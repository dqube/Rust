//! OpenAPI documentation and Scalar UI integration.

use axum::Router;

/// Build a router that serves the Scalar API reference UI at `/scalar`.
///
/// Requires a JSON spec to be served at `openapi_json_path` (e.g.
/// `/api-docs/openapi.json`).
pub fn scalar_router(openapi_json_path: &str) -> Router {
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>API Reference</title>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
</head>
<body>
    <script id="api-reference" data-url="{}"></script>
    <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
</body>
</html>"#,
        openapi_json_path
    );

    Router::new().route(
        "/scalar",
        axum::routing::get(move || {
            let html = html.clone();
            async move {
                (
                    axum::http::StatusCode::OK,
                    [(axum::http::header::CONTENT_TYPE, "text/html")],
                    html,
                )
            }
        }),
    )
}

/// Build a router that serves an OpenAPI JSON spec at
/// `/api-docs/openapi.json`.
pub fn openapi_json_route<O: utoipa::OpenApi>() -> Router {
    let spec = O::openapi().to_json().unwrap_or_default();
    Router::new().route(
        "/api-docs/openapi.json",
        axum::routing::get(move || {
            let spec = spec.clone();
            async move {
                (
                    axum::http::StatusCode::OK,
                    [(axum::http::header::CONTENT_TYPE, "application/json")],
                    spec,
                )
            }
        }),
    )
}
