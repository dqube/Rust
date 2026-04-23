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
use crate::api::rest::catalog;
use crate::api::rest::catalog_summary::get_catalog_summary;
use crate::api::rest::customers;
use crate::api::rest::employees;
use crate::api::rest::orders;
use crate::api::rest::products;
use crate::api::rest::shared;
use crate::api::rest::suppliers;
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

    // Customer CRUD, loyalty, addresses, avatar, profile/KYC, wishlist
    let customer_routes = Router::new()
        // CRUD
        .route("/admin/customers", post(customers::create_customer).get(customers::list_customers))
        .route("/admin/customers/ensure", post(customers::ensure_customer_profile))
        .route("/admin/customers/by-user/{user_id}", get(customers::get_customer_by_user_id))
        .route("/admin/customers/{id}", get(customers::get_customer))
        .route("/admin/customers/{id}/info", put(customers::update_customer_info))
        // Loyalty
        .route("/admin/customers/{id}/loyalty/add", post(customers::add_loyalty_points))
        .route("/admin/customers/{id}/loyalty/redeem", post(customers::redeem_loyalty_points))
        // Addresses
        .route("/admin/customers/{id}/addresses", post(customers::add_customer_address))
        .route("/admin/customers/{id}/addresses/{address_id}", put(customers::update_customer_address).delete(customers::remove_customer_address))
        .route("/admin/customers/{id}/addresses/{address_id}/set-default", put(customers::set_default_customer_address))
        // Avatar
        .route("/admin/customers/{id}/avatar-upload-url", post(customers::request_avatar_upload_url))
        .route("/admin/customers/{id}/confirm-avatar", post(customers::confirm_avatar_upload))
        .route("/admin/customers/{id}/avatar-url", get(customers::get_customer_avatar_url))
        // Profile
        .route("/admin/customers/{id}/profile", post(customers::create_customer_profile).get(customers::get_customer_profile).put(customers::update_customer_profile))
        .route("/admin/customers/{id}/profile/notifications", put(customers::update_notification_preferences))
        // KYC
        .route("/admin/customers/{id}/kyc/documents", post(customers::submit_kyc_document))
        .route("/admin/customers/{id}/kyc/document-upload-url", post(customers::request_kyc_document_upload_url))
        .route("/admin/customers/{id}/kyc/submit-review", post(customers::submit_for_kyc_review))
        .route("/admin/customers/{id}/kyc/verify", post(customers::verify_kyc))
        .route("/admin/customers/{id}/kyc/reject", post(customers::reject_kyc))
        // Wishlist
        .route("/admin/customers/{id}/wishlist", get(customers::get_wishlist).delete(customers::clear_wishlist))
        .route("/admin/customers/{id}/wishlist/items", post(customers::add_to_wishlist))
        .route("/admin/customers/{id}/wishlist/items/{product_id}", delete(customers::remove_from_wishlist));

    // Employee CRUD, departments, designations, avatar
    let employee_routes = Router::new()
        // Employees
        .route("/admin/employees", post(employees::create_employee).get(employees::list_employees))
        .route("/admin/employees/by-user/{user_id}", get(employees::get_employee_by_user_id))
        .route("/admin/employees/by-code/{code}", get(employees::get_employee_by_code))
        .route("/admin/employees/{id}", get(employees::get_employee).put(employees::update_employee))
        .route("/admin/employees/{id}/terminate", put(employees::terminate_employee))
        .route("/admin/employees/{id}/reactivate", put(employees::reactivate_employee))
        .route("/admin/employees/{id}/assign-store", put(employees::assign_to_store))
        // Avatar
        .route("/admin/employees/{id}/avatar-upload-url", post(employees::request_avatar_upload_url))
        .route("/admin/employees/{id}/confirm-avatar", post(employees::confirm_avatar_upload))
        .route("/admin/employees/{id}/avatar", delete(employees::delete_avatar).get(employees::get_avatar_url))
        // Departments
        .route("/admin/employees/departments", post(employees::create_department).get(employees::list_departments))
        .route("/admin/employees/departments/{id}", get(employees::get_department).put(employees::update_department))
        // Designations
        .route("/admin/employees/designations", post(employees::create_designation).get(employees::list_designations))
        .route("/admin/employees/designations/{id}", get(employees::get_designation).put(employees::update_designation));

    // Catalog — products, categories, brands, tax configurations (REST → gRPC pass-through)
    let catalog_routes = Router::new()
        // Products
        .route("/admin/catalog/products", post(catalog::create_product).get(catalog::list_products))
        .route("/admin/catalog/products/{id}", get(catalog::get_product).put(catalog::update_product))
        .route("/admin/catalog/products/{id}/discontinue", put(catalog::discontinue_product))
        .route("/admin/catalog/products/{id}/reactivate", put(catalog::reactivate_product))
        .route("/admin/catalog/products/{id}/pricing", put(catalog::update_product_pricing))
        .route("/admin/catalog/products/{id}/brand", put(catalog::assign_product_brand))
        .route("/admin/catalog/products/{id}/dimensions", put(catalog::set_product_dimensions))
        .route("/admin/catalog/products/{id}/specifications", put(catalog::set_product_specifications))
        .route("/admin/catalog/products/{id}/tags", put(catalog::set_product_tags))
        .route("/admin/catalog/products/{id}/tax-configurations", put(catalog::set_product_tax_configurations))
        .route("/admin/catalog/products/{id}/variants", post(catalog::add_product_variant))
        .route("/admin/catalog/products/{id}/variants/{variant_id}", put(catalog::update_product_variant).delete(catalog::remove_product_variant))
        .route("/admin/catalog/products/{id}/variants/{variant_id}/set-default", put(catalog::set_default_variant))
        .route("/admin/catalog/products/{id}/image-upload-url", post(catalog::request_product_image_upload_url))
        .route("/admin/catalog/products/{id}/confirm-image", post(catalog::confirm_product_image_upload))
        .route("/admin/catalog/products/{id}/images/{image_id}", delete(catalog::delete_product_image))
        // Categories
        .route("/admin/catalog/categories", post(catalog::create_category).get(catalog::list_categories))
        .route("/admin/catalog/categories/{id}", get(catalog::get_category).put(catalog::update_category).delete(catalog::delete_category))
        .route("/admin/catalog/categories/{id}/image-upload-url", post(catalog::request_category_image_upload_url))
        .route("/admin/catalog/categories/{id}/confirm-image", post(catalog::confirm_category_image_upload))
        // Brands
        .route("/admin/catalog/brands", post(catalog::create_brand).get(catalog::list_brands))
        .route("/admin/catalog/brands/{id}", get(catalog::get_brand).put(catalog::update_brand))
        .route("/admin/catalog/brands/{id}/activate", put(catalog::activate_brand))
        .route("/admin/catalog/brands/{id}/deactivate", put(catalog::deactivate_brand))
        // Tax configurations
        .route("/admin/catalog/tax-configurations", post(catalog::create_tax_configuration).get(catalog::list_tax_configurations))
        .route("/admin/catalog/tax-configurations/{id}", get(catalog::get_tax_configuration).put(catalog::update_tax_configuration).delete(catalog::delete_tax_configuration))
        .route("/admin/catalog/tax-configurations/{id}/activate", put(catalog::activate_tax_configuration))
        .route("/admin/catalog/tax-configurations/{id}/deactivate", put(catalog::deactivate_tax_configuration))
        .route("/admin/catalog/tax-configurations/applicable", get(catalog::get_applicable_tax_configurations))
        .route("/admin/catalog/tax-configurations/calculate", post(catalog::calculate_tax));

    // Supplier CRUD, addresses, contacts, documents, products, purchase orders
    let supplier_routes = Router::new()
        // Suppliers
        .route("/admin/suppliers", post(suppliers::create_supplier).get(suppliers::list_suppliers))
        .route("/admin/suppliers/{id}", get(suppliers::get_supplier).put(suppliers::update_supplier).delete(suppliers::delete_supplier))
        .route("/admin/suppliers/{id}/activate", put(suppliers::activate_supplier))
        .route("/admin/suppliers/{id}/deactivate", put(suppliers::deactivate_supplier))
        .route("/admin/suppliers/{id}/status", put(suppliers::update_supplier_status))
        .route("/admin/suppliers/{id}/onboarding-status", put(suppliers::update_onboarding_status))
        // Addresses
        .route("/admin/suppliers/{id}/addresses", get(suppliers::get_supplier_addresses))
        // Contacts
        .route("/admin/suppliers/{id}/contacts", get(suppliers::get_supplier_contacts).post(suppliers::create_supplier_contact))
        // Documents
        .route("/admin/suppliers/{id}/documents", get(suppliers::get_supplier_documents))
        .route("/admin/suppliers/{id}/documents/upload-url", post(suppliers::request_document_upload_url))
        .route("/admin/suppliers/{id}/documents/confirm", post(suppliers::confirm_document_upload))
        .route("/admin/suppliers/{id}/documents/{document_id}", delete(suppliers::delete_supplier_document))
        // Supplier products
        .route("/admin/suppliers/{id}/products", get(suppliers::list_supplier_products).post(suppliers::add_supplier_product))
        .route("/admin/suppliers/{id}/products/{supplier_product_id}", delete(suppliers::remove_supplier_product))
        // Purchase orders
        .route("/admin/purchase-orders", post(suppliers::create_purchase_order).get(suppliers::list_purchase_orders))
        .route("/admin/purchase-orders/{id}", get(suppliers::get_purchase_order))
        .route("/admin/purchase-orders/{id}/submit", put(suppliers::submit_purchase_order))
        .route("/admin/purchase-orders/{id}/cancel", put(suppliers::cancel_purchase_order));

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
        .merge(customer_routes)
        .merge(employee_routes)
        .merge(supplier_routes)
        .merge(catalog_routes)
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
