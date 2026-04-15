//! Generic OpenAPI / Scalar router for BFF gateways.
//!
//! Serves the Scalar UI at `/scalar` and the raw spec JSON at
//! `/api-docs/openapi.json`. The spec is embedded directly in the Scalar
//! HTML — no external CDN request is needed at browser load time.
//!
//! Feature-gated on `axum-response`.

use axum::Router;
use utoipa_scalar::{Scalar, Servable as _};

/// Mount the Scalar UI at `/scalar` and serve the OpenAPI JSON at
/// `/api-docs/openapi.json`.
///
/// ```ignore
/// let app = Router::new()
///     // ... your routes ...
///     .merge(ddd_bff::openapi::openapi_router(my_spec));
/// ```
pub fn openapi_router(spec: serde_json::Value) -> Router {
    let spec_json = spec.to_string();

    // Scalar UI with the spec embedded — uses the bundled scalar.html
    // from the utoipa-scalar crate; no CDN JavaScript reference.
    let scalar_router = Router::from(Scalar::with_url("/scalar", spec));

    scalar_router.route(
        "/api-docs/openapi.json",
        axum::routing::get(move || {
            let json = spec_json.clone();
            async move {
                (
                    axum::http::StatusCode::OK,
                    [(axum::http::header::CONTENT_TYPE, "application/json")],
                    json,
                )
            }
        }),
    )
}
