//! Pre-configured middleware layers for REST APIs.

use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::request_id::{MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use http::Request;
use tracing::Span;

// ─── Request ID ──────────────────────────────────────────────────────────────

/// Generates a UUID v7 request ID.
#[derive(Clone, Copy)]
pub struct UuidRequestId;

impl MakeRequestId for UuidRequestId {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let id = uuid::Uuid::now_v7().to_string();
        id.parse().ok().map(RequestId::new)
    }
}

/// Create a [`SetRequestIdLayer`] that generates UUID v7 request IDs in the
/// `x-request-id` header.
pub fn request_id_layer() -> SetRequestIdLayer<UuidRequestId> {
    SetRequestIdLayer::x_request_id(UuidRequestId)
}

/// Create a [`PropagateRequestIdLayer`] that copies `x-request-id` to the
/// response.
pub fn propagate_request_id_layer() -> PropagateRequestIdLayer {
    PropagateRequestIdLayer::x_request_id()
}

// ─── Tracing ─────────────────────────────────────────────────────────────────

/// Create a [`TraceLayer`] that logs method, URI, status, and latency.
#[allow(clippy::type_complexity)]
pub fn tracing_layer(
) -> TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    impl Fn(&Request<axum::body::Body>) -> Span + Clone,
    tower_http::trace::DefaultOnRequest,
    impl Fn(&http::Response<axum::body::Body>, std::time::Duration, &Span) + Clone,
> {
    TraceLayer::new_for_http()
        .make_span_with(|request: &Request<_>| {
            let request_id = request
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");
            tracing::info_span!(
                "http_request",
                method = %request.method(),
                uri = %request.uri(),
                request_id = %request_id,
            )
        })
        .on_response(
            |response: &http::Response<_>, latency: std::time::Duration, _span: &Span| {
                tracing::info!(
                    status = %response.status(),
                    latency_ms = latency.as_millis(),
                    "response"
                );
            },
        )
}

// ─── CORS ────────────────────────────────────────────────────────────────────

/// Create a [`CorsLayer`].
///
/// If the `CORS_ALLOWED_ORIGINS` environment variable is set (comma-separated),
/// those origins are used; otherwise all origins are allowed.
pub fn cors_layer() -> CorsLayer {
    match std::env::var("CORS_ALLOWED_ORIGINS") {
        Ok(origins) => {
            let origins: Vec<_> = origins
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods(Any)
                .allow_headers(Any)
        }
        Err(_) => CorsLayer::permissive(),
    }
}

// ─── Compression ─────────────────────────────────────────────────────────────

/// Create a [`CompressionLayer`] with all supported algorithms enabled.
pub fn compression_layer() -> CompressionLayer {
    CompressionLayer::new()
}
