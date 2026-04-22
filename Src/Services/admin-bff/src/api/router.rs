use axum::{middleware as axum_mw, routing::{delete, get, post, put}, Router};
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::timeout::TimeoutLayer;
use utoipa::OpenApi;

use ddd_bff::metrics::metrics_handler;
use ddd_bff::middleware::axum_observability::{observability_middleware, ObservabilityState};
use ddd_bff::openapi::{inject_routes, merged_openapi, openapi_router};
use ddd_bff::transcode::fallback_handler;
use ddd_shared_kernel::jwt::StandardClaims;

use crate::api::openapi::AdminApiDoc;
use crate::api::openapi_routes::API_ROUTES;
use crate::api::rest::auth;
use crate::api::rest::batch_orders::batch_get_orders;
use crate::api::rest::catalog_summary::get_catalog_summary;
use crate::api::rest::orders;
use crate::api::rest::products;
use crate::api::rest::shared;
use crate::application::state::AppState;
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
        .route("/admin/catalog/summary", get(get_catalog_summary));

    // Order batch aggregation (gRPC fan-out — registered before order_routes so /batch wins)
    let aggregation_routes = Router::new()
        .route("/admin/orders/batch", post(batch_get_orders));

    // Order CRUD (REST → gRPC pass-through)
    let order_routes = Router::new()
        .route("/admin/orders",             post(orders::create_order).get(orders::list_orders))
        .route("/admin/orders/{id}",         get(orders::get_order))
        .route("/admin/orders/{id}/confirm", put(orders::confirm_order))
        .route("/admin/orders/{id}/cancel",  put(orders::cancel_order));

    // Shared reference data (REST → gRPC pass-through)
    let shared_routes = Router::new()
        // Currencies
        .route("/admin/shared/currencies", post(shared::create_currency).get(shared::list_currencies))
        .route("/admin/shared/currencies/{code}", get(shared::get_currency).put(shared::update_currency).delete(shared::delete_currency))
        .route("/admin/shared/currencies/{code}/activate", put(shared::activate_currency))
        .route("/admin/shared/currencies/{code}/deactivate", put(shared::deactivate_currency))
        // Countries
        .route("/admin/shared/countries", post(shared::create_country).get(shared::list_countries))
        .route("/admin/shared/countries/{code}", get(shared::get_country).put(shared::update_country).delete(shared::delete_country))
        .route("/admin/shared/countries/{code}/activate", put(shared::activate_country))
        .route("/admin/shared/countries/{code}/deactivate", put(shared::deactivate_country))
        .route("/admin/shared/currencies/{code}/countries", get(shared::list_countries_by_currency))
        // States
        .route("/admin/shared/states", post(shared::create_state).get(shared::list_states))
        .route("/admin/shared/states/{code}", get(shared::get_state).put(shared::update_state).delete(shared::delete_state))
        .route("/admin/shared/states/{code}/activate", put(shared::activate_state))
        .route("/admin/shared/states/{code}/deactivate", put(shared::deactivate_state))
        .route("/admin/shared/countries/{code}/states", get(shared::list_states_by_country))
        // Cities
        .route("/admin/shared/cities", post(shared::create_city).get(shared::list_cities))
        .route("/admin/shared/cities/{code}", get(shared::get_city).put(shared::update_city).delete(shared::delete_city))
        .route("/admin/shared/cities/{code}/activate", put(shared::activate_city))
        .route("/admin/shared/cities/{code}/deactivate", put(shared::deactivate_city))
        .route("/admin/shared/states/{code}/cities", get(shared::list_cities_by_state))
        // Pincodes
        .route("/admin/shared/pincodes", post(shared::create_pincode).get(shared::list_pincodes))
        .route("/admin/shared/pincodes/{code}", get(shared::get_pincode).put(shared::update_pincode).delete(shared::delete_pincode))
        .route("/admin/shared/pincodes/{code}/activate", put(shared::activate_pincode))
        .route("/admin/shared/pincodes/{code}/deactivate", put(shared::deactivate_pincode))
        .route("/admin/shared/cities/{code}/pincodes", get(shared::list_pincodes_by_city));

    // Auth (REST → gRPC pass-through)
    let auth_routes = Router::new()
        // Auth flows
        .route("/admin/auth/login",           post(auth::login))
        .route("/admin/auth/register",        post(auth::register))
        .route("/admin/auth/refresh",         post(auth::refresh_token))
        .route("/admin/auth/logout",          post(auth::logout))
        .route("/admin/auth/change-password", post(auth::change_password))
        .route("/admin/auth/forgot-password", post(auth::forgot_password))
        .route("/admin/auth/reset-password",  post(auth::reset_password))
        .route("/admin/auth/check-permission", post(auth::check_permission))
        .route("/admin/auth/role-permissions", post(auth::get_role_permissions))
        // Users
        .route("/admin/auth/users",                        get(auth::list_users))
        .route("/admin/auth/users/{user_id}",              get(auth::get_user))
        .route("/admin/auth/users/by-email/{email}",       get(auth::get_user_by_email))
        .route("/admin/auth/users/{user_id}/activate",     post(auth::activate_user))
        .route("/admin/auth/users/{user_id}/deactivate",   post(auth::deactivate_user))
        .route("/admin/auth/users/{user_id}/change-password-admin", post(auth::change_password_admin))
        .route("/admin/auth/users/{user_id}/roles",        get(auth::list_user_roles).post(auth::assign_role))
        // Roles
        .route("/admin/auth/roles",                                post(auth::create_role).get(auth::list_roles))
        .route("/admin/auth/roles/{role_id}/permissions",          get(auth::get_role_permissions_by_id).post(auth::add_role_permission))
        .route("/admin/auth/roles/{role_id}/permissions/{permission}", delete(auth::remove_role_permission))
        // User-role link
        .route("/admin/auth/user-roles/{user_role_id}", delete(auth::remove_user_role));

    // Group /admin/* routes and guard them with JWT auth when configured.
    let mut admin_routes = Router::new()
        .merge(product_routes)
        .merge(aggregation_routes)
        .merge(order_routes)
        .merge(shared_routes)
        .merge(auth_routes)
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
        redact_fields: std::sync::Arc::new(state.config.bff.redact_fields.clone()),
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
        .layer(TimeoutLayer::with_status_code(axum::http::StatusCode::REQUEST_TIMEOUT, state.config.bff.request_timeout))
}
