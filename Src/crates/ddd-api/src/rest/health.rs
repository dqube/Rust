//! Kubernetes-style health and readiness probes.
//!
//! # Usage
//!
//! ```rust,ignore
//! use ddd_api::rest::health::{health_router, HealthCheck, HealthCheckRegistry};
//!
//! // Implement HealthCheck for each dependency.
//! struct DbCheck { pool: PgPool }
//!
//! #[async_trait::async_trait]
//! impl HealthCheck for DbCheck {
//!     fn name(&self) -> &str { "postgres" }
//!     async fn check(&self) -> Result<(), String> {
//!         sqlx::query("SELECT 1").execute(&self.pool).await
//!             .map(|_| ())
//!             .map_err(|e| e.to_string())
//!     }
//! }
//!
//! // Register checks and mount the router.
//! let mut registry = HealthCheckRegistry::new();
//! registry.register(Arc::new(DbCheck { pool }));
//!
//! let app = Router::new()
//!     .merge(health_router(Arc::new(registry)));
//! ```
//!
//! ## Endpoints
//!
//! | Path | Purpose | Behaviour |
//! |------|---------|-----------|
//! | `GET /health` | Liveness | Always returns `200 OK`. If the process can respond, it's alive. |
//! | `GET /ready` | Readiness | Runs every registered [`HealthCheck`]. Returns `200` when all pass, `503` otherwise. |

use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

// ─── HealthCheck trait ───────────────────────────────────────────────────────

/// A single dependency health check (database, message broker, cache, etc.).
///
/// Implement this trait for each external dependency your service relies on
/// and register it with a [`HealthCheckRegistry`].
#[async_trait::async_trait]
pub trait HealthCheck: Send + Sync {
    /// Short human-readable name (e.g. `"postgres"`, `"nats"`, `"redis"`).
    fn name(&self) -> &str;

    /// Perform the check.
    ///
    /// Return `Ok(())` when the dependency is reachable and functional, or
    /// `Err(message)` with a description of the failure.
    async fn check(&self) -> Result<(), String>;
}

// ─── HealthCheckRegistry ─────────────────────────────────────────────────────

/// Collects [`HealthCheck`] implementations that the readiness endpoint should
/// query.
#[derive(Default)]
pub struct HealthCheckRegistry {
    checks: Vec<Arc<dyn HealthCheck>>,
}

impl HealthCheckRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a health check.
    pub fn register(&mut self, check: Arc<dyn HealthCheck>) {
        self.checks.push(check);
    }
}

// ─── Response DTOs ───────────────────────────────────────────────────────────

/// Top-level response body for `/health` and `/ready`.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// `"healthy"` or `"unhealthy"`.
    pub status: &'static str,
    /// Individual check results (empty for `/health`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<CheckResult>,
}

/// Result of a single dependency check.
#[derive(Debug, Serialize)]
pub struct CheckResult {
    /// Check name.
    pub name: String,
    /// `"up"` or `"down"`.
    pub status: &'static str,
    /// Error message when `status` is `"down"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ─── Router constructor ──────────────────────────────────────────────────────

/// Build an Axum [`Router`] with `/health` and `/ready` endpoints.
///
/// Pass the returned router to [`RestServer::merge`] or merge it into your
/// own application router.
pub fn health_router(registry: Arc<HealthCheckRegistry>) -> Router {
    Router::new()
        .route("/health", get(liveness))
        .route("/ready", get(readiness))
        .with_state(registry)
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// `GET /health` — liveness probe.
///
/// Always returns `200 OK` with `{"status":"healthy"}`.  If the process can
/// serve this request, it is alive.
async fn liveness() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy",
        checks: Vec::new(),
    })
}

/// `GET /ready` — readiness probe.
///
/// Runs all registered health checks concurrently.  Returns `200` when every
/// check passes, or `503 Service Unavailable` with details when any fail.
async fn readiness(State(registry): State<Arc<HealthCheckRegistry>>) -> impl IntoResponse {
    let futures: Vec<_> = registry
        .checks
        .iter()
        .map(|c| {
            let check = Arc::clone(c);
            async move {
                let result = check.check().await;
                CheckResult {
                    name: check.name().to_owned(),
                    status: if result.is_ok() { "up" } else { "down" },
                    error: result.err(),
                }
            }
        })
        .collect();

    let checks = futures::future::join_all(futures).await;
    let all_up = checks.iter().all(|c| c.status == "up");

    let status_code = if all_up {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let body = HealthResponse {
        status: if all_up { "healthy" } else { "unhealthy" },
        checks,
    };

    (status_code, Json(body))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    struct UpCheck;
    #[async_trait::async_trait]
    impl HealthCheck for UpCheck {
        fn name(&self) -> &str {
            "always_up"
        }
        async fn check(&self) -> Result<(), String> {
            Ok(())
        }
    }

    struct DownCheck;
    #[async_trait::async_trait]
    impl HealthCheck for DownCheck {
        fn name(&self) -> &str {
            "always_down"
        }
        async fn check(&self) -> Result<(), String> {
            Err("connection refused".into())
        }
    }

    #[tokio::test]
    async fn liveness_always_200() {
        let registry = Arc::new(HealthCheckRegistry::new());
        let app = health_router(registry);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "healthy");
    }

    #[tokio::test]
    async fn ready_all_up_returns_200() {
        let mut registry = HealthCheckRegistry::new();
        registry.register(Arc::new(UpCheck));
        let app = health_router(Arc::new(registry));

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "healthy");
        assert_eq!(json["checks"][0]["name"], "always_up");
        assert_eq!(json["checks"][0]["status"], "up");
    }

    #[tokio::test]
    async fn ready_one_down_returns_503() {
        let mut registry = HealthCheckRegistry::new();
        registry.register(Arc::new(UpCheck));
        registry.register(Arc::new(DownCheck));
        let app = health_router(Arc::new(registry));

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "unhealthy");

        let down = json["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "always_down")
            .unwrap();
        assert_eq!(down["status"], "down");
        assert_eq!(down["error"], "connection refused");
    }

    #[tokio::test]
    async fn ready_no_checks_returns_200() {
        let registry = Arc::new(HealthCheckRegistry::new());
        let app = health_router(registry);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }
}
