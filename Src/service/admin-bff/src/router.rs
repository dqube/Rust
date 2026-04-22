use axum::{middleware as axum_mw, routing::{get, post, put}, Router};
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::timeout::TimeoutLayer;
use utoipa::OpenApi;

use ddd_bff::metrics::metrics_handler;
use ddd_bff::middleware::axum_observability::{observability_middleware, ObservabilityState};
use ddd_bff::openapi::{inject_routes, merged_openapi, openapi_router};
use ddd_bff::transcode::fallback_handler;
use ddd_shared_kernel::jwt::StandardClaims;

use crate::aggregation::batch_get_orders;
use crate::handlers::orders;
use crate::handlers::products;
use crate::openapi::AdminApiDoc;
use crate::openapi_routes::API_ROUTES;
use crate::state::AppState;
use ddd_bff::middleware::axum_auth::jwt_auth_layer;

pub async fn build_router(state: AppState) -> Router {
    // ── OpenAPI (base + downstream order-service merged) ─────────────────
    let base_spec = serde_json::to_value(AdminApiDoc::openapi())
        .unwrap_or_else(|_| serde_json::json!({}));
    let downstream_spec_url = format!(
        "{}/api-docs/openapi.json",
        state.config.services.order_service
    );
    let mut merged_spec = merged_openapi(base_spec, &downstream_spec_url, "/admin/orders").await;
    inject_routes(&mut merged_spec, API_ROUTES);

    // ── Routes ───────────────────────────────────────────────────────────

    // Product CRUD + image upload (REST → gRPC pass-through)
    let product_routes = Router::new()
        .route("/admin/products", post(products::create_product).get(products::list_products))
        .route("/admin/products/{id}", get(products::get_product))
        .route("/admin/products/{id}/stock", put(products::update_stock))
        .route("/admin/products/{id}/deactivate", put(products::deactivate_product))
        .route("/admin/products/{id}/image-upload-url", post(products::request_image_upload_url))
        .route("/admin/products/{id}/confirm-image", post(products::confirm_image_upload))
        .route("/admin/catalog/summary", get(crate::handlers::aggregation::get_catalog_summary));

    // Order batch aggregation (gRPC fan-out — registered before order_routes so /batch wins)
    let aggregation_routes = Router::new()
        .route("/admin/orders/batch", post(batch_get_orders));

    // Order CRUD (REST → gRPC pass-through)
    let order_routes = Router::new()
        .route("/admin/orders",             post(orders::create_order).get(orders::list_orders))
        .route("/admin/orders/{id}",         get(orders::get_order))
        .route("/admin/orders/{id}/confirm", put(orders::confirm_order))
        .route("/admin/orders/{id}/cancel",  put(orders::cancel_order));

    // Group /admin/* routes and guard them with JWT auth when configured.
    // Group /admin/* routes and guard them with JWT auth when configured.
    let mut admin_routes = Router::new()
        .merge(product_routes)
        .merge(aggregation_routes)
        .merge(order_routes)
        // Supply AppState to all admin handlers before applying layers that expect Router<()>
        .with_state(state.clone());
        
    if let Some(validator) = state.jwt_validator.clone() {
        admin_routes = admin_routes.layer(axum_mw::from_fn_with_state(
            validator,
            jwt_auth_layer::<StandardClaims>,
        ));
    }

    // Observability state
    let log_bodies = std::env::var("LOG_REQUEST_BODIES")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    let obs_state = ObservabilityState {
        redact_fields: std::sync::Arc::new(state.config.redact_fields.clone()),
        log_bodies,
    };

    Router::new()
        // Protected admin surface
        .merge(admin_routes)
        // Health
        .route("/health", get(|| async { "ok" }))
        // Metrics
        .route("/metrics", get(metrics_handler))
        // OpenAPI / Scalar (merged spec)
        .merge(openapi_router(merged_spec))
        // Fallback → Problem Details 404
        .fallback(fallback_handler)
        // Observability middleware
        .layer(axum_mw::from_fn_with_state(obs_state, observability_middleware))
        // Catch panics
        .layer(CatchPanicLayer::new())
        // Per-request timeout — outermost layer so it covers the full lifecycle
        .layer(TimeoutLayer::with_status_code(axum::http::StatusCode::REQUEST_TIMEOUT, state.config.request_timeout))
}
