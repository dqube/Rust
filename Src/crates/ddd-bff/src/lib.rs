//! # ddd-bff
//!
//! Reusable building blocks for **Backend-for-Frontend** gateways.
//!
//! `ddd-bff` is a library crate. It provides the pieces needed to stand
//! up a REST gateway in front of a fleet of gRPC services without
//! coupling to any specific service:
//!
//! - **Clients** ([`clients`]) — generic [`clients::GrpcClientPool`]
//!   keyed by service name and a [`clients::ResilientChannel`] wrapper.
//! - **Config** ([`config`]) — [`config::BffConfig`] with universal
//!   settings (host, ports, resilience, redaction, OTLP, shutdown).
//! - **Metrics** ([`metrics`]) — Prometheus singleton + encode helper.
//! - **Middleware** ([`middleware`]) — [`middleware::redaction`] for
//!   logged-payload field scrubbing; [`middleware::axum_observability`]
//!   for axum request logging + metrics (requires `axum-response`);
//!   optional JWT bearer-token validation (`middleware::axum_auth`,
//!   feature `jwt`).
//! - **OpenAPI** ([`openapi`]) — declarative endpoint catalogue
//!   ([`openapi::ApiRoute`]), spec injection, Scalar router, and
//!   downstream spec merge (requires `axum-response` for router/merge).
//! - **Proxy** ([`proxy`]) — generic HTTP reverse proxy for REST
//!   downstreams (requires `axum-response`).
//! - **Transcode** ([`transcode`]) — gRPC `Status` → [`AppError`] →
//!   RFC 9457 [`transcode::ProblemDetail`] error-mapping pipeline.
//! - **Edge** ([`edge`]) — graceful shutdown signal handlers.
//!
//! Consumers supply their own `main.rs`, downstream service URLs, and
//! router wiring. See the `admin-bff` service for a reference consumer.
//!
//! [`AppError`]: ddd_shared_kernel::AppError
//!
//! ## Feature flags
//!
//! | Feature | Enables |
//! |---------|---------|
//! | `axum-response` | `axum::IntoResponse` for [`transcode::ProblemDetail`], the axum observability / tracing / audit middleware, the Prometheus `metrics_handler`, the generic OpenAPI router + downstream spec merge, the HTTP [`proxy`], and the [`prelude`] module. Required for any axum-based REST layer. |
//! | `jwt` | JWT bearer-token validation middleware (`middleware::axum_auth`). Implies `axum-response` and `ddd-shared-kernel/jwt`. |

pub mod clients;
pub mod config;
pub mod edge;
pub mod metrics;
pub mod middleware;
pub mod openapi;
pub mod transcode;

#[cfg(feature = "axum-response")]
pub mod proxy;

// ─── Crate-root re-exports ───────────────────────────────────────────────────

// Clients
pub use clients::{GrpcClientPool, ResilientChannel};

// Config
pub use config::{env_or, BffConfig, ResilienceConfig};

// Edge
pub use edge::shutdown::wait_for_shutdown_signal;

// Metrics
pub use metrics::BFF_METRICS;
#[cfg(feature = "axum-response")]
pub use metrics::metrics_handler;

// Middleware
pub use middleware::redaction::{redact_json, redact_json_string};

// Transcode — error types and mapping helpers
pub use transcode::{
    app_error_to_problem, grpc_status_to_app_error, FieldViolation, ProblemDetail,
};

// OpenAPI — declarative catalogue helpers (always available)
pub use openapi::{inject_routes, ApiRoute, RouteKind};
#[cfg(feature = "axum-response")]
pub use openapi::{merged_openapi, openapi_router};

// Proxy (axum-response)
#[cfg(feature = "axum-response")]
pub use proxy::{proxy_handler, ProxyState};

/// Common imports for BFF handlers.
///
/// ```ignore
/// use ddd_bff::prelude::*;
/// ```
///
/// Bundles request-context types, the task-local trace propagator, the audit
/// emitter, and the RFC 9457 error types — the set every REST → gRPC handler
/// needs to propagate trace metadata, emit audit events, and map downstream
/// gRPC failures to Problem Details.
///
/// Glob-importing a prelude is a readability trade-off: it shortens handler
/// imports but hides origin. Use it inside per-service handler modules; avoid
/// it in library crates and wiring files.
#[cfg(feature = "axum-response")]
pub mod prelude {
    pub use crate::middleware::audit::{audit, AuditEvent};
    pub use crate::middleware::axum_observability::{
        observability_middleware, ClientIp, ObservabilityState, RequestTraceContext,
    };
    pub use crate::middleware::tracing_interceptor::{TracingInterceptor, TRACE_CTX};
    pub use crate::transcode::{GrpcIntoProblem, IntoProblem, ProblemDetail};
}
