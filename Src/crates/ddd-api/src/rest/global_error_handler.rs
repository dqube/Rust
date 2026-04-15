//! Global exception handler for REST APIs.
//!
//! Provides two layers that should wrap the entire router:
//!
//! 1. **Panic catcher** — converts panics into `500 Internal Server Error`
//!    responses in Problem Details format instead of dropping the connection.
//! 2. **Fallback handler** — catches 404/405 from unmatched routes and returns
//!    Problem Details instead of Axum's default plain-text body.
//!
//! # Usage
//!
//! ```rust,ignore
//! use ddd_api::rest::global_error_handler::{catch_panic_layer, fallback_handler};
//!
//! let app = Router::new()
//!     .route("/items", get(list_items))
//!     .fallback(fallback_handler)
//!     .layer(catch_panic_layer());
//! ```

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::any::Any;
use tower_http::catch_panic::CatchPanicLayer;

use super::problem_details::ProblemDetail;

// ─── Panic catcher layer ─────────────────────────────────────────────────────

/// Returns a [`CatchPanicLayer`] that renders panics as RFC 9457 Problem
/// Details responses with status `500`.
///
/// This MUST be the outermost layer on the router so it can catch panics from
/// any inner middleware or handler.
pub fn catch_panic_layer() -> CatchPanicLayer<PanicResponseMapper> {
    CatchPanicLayer::custom(PanicResponseMapper)
}

/// Maps a panic payload to a Problem Details response.
#[derive(Debug, Clone, Copy)]
pub struct PanicResponseMapper;

impl tower_http::catch_panic::ResponseForPanic for PanicResponseMapper {
    type ResponseBody = axum::body::Body;

    fn response_for_panic(
        &mut self,
        err: Box<dyn Any + Send + 'static>,
    ) -> axum::http::Response<Self::ResponseBody> {
        let detail = if let Some(s) = err.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = err.downcast_ref::<&str>() {
            (*s).to_owned()
        } else {
            "An unexpected error occurred".to_owned()
        };

        tracing::error!(detail = %detail, "handler panicked");

        // Do not leak the raw panic message to clients in production.
        let pd = ProblemDetail::new(500, "Internal Server Error", "An unexpected error occurred")
            .with_type("urn:problem-type:internal-error");

        pd.into_response()
    }
}

// ─── Fallback handler ────────────────────────────────────────────────────────

/// Axum fallback handler that returns a Problem Details `404` for any
/// unmatched route.
///
/// Register with `Router::fallback(fallback_handler)`.
pub async fn fallback_handler(
    method: axum::http::Method,
    uri: axum::http::Uri,
) -> impl IntoResponse {
    // Log at debug level — 404s from scanners/bots are noisy.
    tracing::debug!(method = %method, uri = %uri, "no matching route");

    let pd = ProblemDetail::new(
        404,
        "Not Found",
        format!("No route matched {method} {uri}"),
    )
    .with_type("urn:problem-type:not-found");

    pd
}

// ─── Unhandled rejection handler ─────────────────────────────────────────────

/// Convert an unhandled rejection (e.g. missing `Content-Type`, payload too
/// large) into a Problem Details response.
///
/// Use as the argument to Axum's `.with_state()` error handler, or call
/// manually when you need to turn an arbitrary status code into a
/// Problem Details body.
pub fn status_to_problem_detail(status: StatusCode) -> Response {
    let pd = ProblemDetail::new(
        status.as_u16(),
        canonical_reason(status),
        canonical_reason(status),
    )
    .with_type(type_for_status(status));

    pd.into_response()
}

fn canonical_reason(status: StatusCode) -> &'static str {
    status.canonical_reason().unwrap_or("Error")
}

fn type_for_status(status: StatusCode) -> &'static str {
    if status.is_client_error() {
        match status {
            StatusCode::BAD_REQUEST => "urn:problem-type:bad-request",
            StatusCode::NOT_FOUND => "urn:problem-type:not-found",
            StatusCode::METHOD_NOT_ALLOWED => "urn:problem-type:method-not-allowed",
            StatusCode::UNAUTHORIZED => "urn:problem-type:unauthorized",
            StatusCode::FORBIDDEN => "urn:problem-type:forbidden",
            StatusCode::CONFLICT => "urn:problem-type:conflict",
            StatusCode::UNPROCESSABLE_ENTITY => "urn:problem-type:validation-error",
            StatusCode::PAYLOAD_TOO_LARGE => "urn:problem-type:payload-too-large",
            StatusCode::UNSUPPORTED_MEDIA_TYPE => "urn:problem-type:unsupported-media-type",
            StatusCode::TOO_MANY_REQUESTS => "urn:problem-type:rate-limit",
            StatusCode::REQUEST_TIMEOUT => "urn:problem-type:timeout",
            _ => "urn:problem-type:client-error",
        }
    } else {
        match status {
            StatusCode::NOT_IMPLEMENTED => "urn:problem-type:not-implemented",
            StatusCode::SERVICE_UNAVAILABLE => "urn:problem-type:service-unavailable",
            StatusCode::GATEWAY_TIMEOUT => "urn:problem-type:timeout",
            StatusCode::BAD_GATEWAY => "urn:problem-type:bad-gateway",
            _ => "urn:problem-type:internal-error",
        }
    }
}

/// Install the global exception handler layers on a router.
///
/// Attaches:
/// - `fallback_handler` for unmatched routes (returns Problem Details 404)
/// - `catch_panic_layer` as the outermost layer (renders panics as 500)
///
/// ```rust,ignore
/// use ddd_api::rest::global_error_handler::install;
///
/// let app = install(Router::new().route("/items", get(list_items)));
/// ```
pub fn install(router: axum::Router) -> axum::Router {
    router.fallback(fallback_handler).layer(catch_panic_layer())
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    #[tokio::test]
    async fn fallback_returns_problem_detail_404() {
        let app = Router::new()
            .route("/exists", get(|| async { "ok" }))
            .fallback(fallback_handler);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/nope")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(ct, "application/problem+json");

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], 404);
        assert_eq!(json["type"], "urn:problem-type:not-found");
    }

    #[tokio::test]
    async fn panic_returns_problem_detail_500() {
        let app = Router::new()
            .route(
                "/boom",
                get(|| async {
                    panic!("test panic");
                    #[allow(unreachable_code)]
                    "unreachable"
                }),
            )
            .layer(catch_panic_layer());

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/boom")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], 500);
        assert_eq!(json["type"], "urn:problem-type:internal-error");
        // Must NOT leak the panic message.
        assert_eq!(json["detail"], "An unexpected error occurred");
    }

    #[tokio::test]
    async fn status_to_problem_detail_maps_correctly() {
        let resp = status_to_problem_detail(StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["type"], "urn:problem-type:method-not-allowed");
    }
}
