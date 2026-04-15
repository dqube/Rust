//! Admin BFF — REST gateway proxying product-service via gRPC and
//! order-service via HTTP, with observability, metrics, and aggregation.
//!
//! ## Ports
//!
//! | Port | Protocol | Purpose |
//! |------|----------|---------|
//! | 3001 | REST     | Admin API + OpenAPI / Scalar docs + Prometheus metrics |
//!
//! - `/admin/products/*` → product-service gRPC (port 50052)
//! - `/admin/orders/*`   → order-service REST  (port 8080)
//! - `/admin/catalog/summary` → aggregation via product-service gRPC

use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::{get, post, put};
use axum::{middleware as axum_mw, Router};
use ddd_bff::clients::GrpcClientPool;
use ddd_bff::middleware::axum_auth::jwt_auth_layer;
use ddd_shared_kernel::jwt::{JwtValidator, StandardClaims};
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::timeout::TimeoutLayer;
use tracing_subscriber::{
    layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};
use utoipa::OpenApi;

use admin_bff::aggregation::{batch_get_orders, AggregationState};
use admin_bff::config::AdminBffConfig;
use admin_bff::handlers::aggregation;
use admin_bff::handlers::orders::{self, OrderClient};
use admin_bff::handlers::products::{self, ProductClient};
use admin_bff::openapi::{openapi_router, AdminApiDoc};
use ddd_bff::metrics::metrics_handler;
use ddd_bff::middleware::axum_observability::{observability_middleware, ObservabilityState};
use ddd_bff::openapi::{inject_routes, merged_openapi, ApiRoute, Param, ResponseSpec, RouteKind, SchemaRef};
use ddd_bff::transcode::fallback_handler;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    // ── Logging ──────────────────────────────────────────────────────────
    // Initialize tracing BEFORE config validation so that config errors
    // and warnings are emitted through the subscriber.
    //
    // JSON formatter by default so log aggregators (Loki, ELK, CloudWatch,
    // Datadog) can parse fields without regex. Set LOG_FORMAT=pretty for
    // human-readable output during local development.
    //
    // TODO(observability): To integrate Sentry or another error tracker,
    // add a sentry-tracing layer here:
    //   .with(sentry_tracing::layer())
    // Requires the `sentry` and `sentry-tracing` crates + a DSN env var.
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    let fmt_layer = match std::env::var("LOG_FORMAT").as_deref() {
        Ok("pretty") => tracing_subscriber::fmt::layer().pretty().boxed(),
        _ => tracing_subscriber::fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(false)
            .boxed(),
    };
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    // ── Configuration ────────────────────────────────────────────────────
    let config = match AdminBffConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "admin-bff configuration is invalid — aborting");
            std::process::exit(1);
        }
    };

    tracing::info!("Starting Admin BFF on {}:{}", config.host, config.port);

    // ── gRPC channel to product-service (via ddd-bff resilient pool) ────
    // ── gRPC channels (product + order) via ddd-bff resilient pool ─────
    let pool = GrpcClientPool::from_services(
        [
            ("product", config.services.product_service.as_str()),
            ("order",   config.services.order_service.as_str()),
        ],
        &config.resilience,
    )
    .expect("failed to build gRPC client pool");

    let product_channel = pool
        .channel("product")
        .expect("product channel registered above");
    let order_channel = pool
        .channel("order")
        .expect("order channel registered above");

    let product_client = Arc::new(ProductClient::new(product_channel));
    let order_client   = Arc::new(OrderClient::new(order_channel.clone()));

    tracing::info!(url = %config.services.product_service, "connected to product-service (lazy)");
    tracing::info!(url = %config.services.order_service,   "connected to order-service (lazy)");

    // ── Observability ────────────────────────────────────────────────────
    let log_bodies = std::env::var("LOG_REQUEST_BODIES")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    let obs_state = ObservabilityState {
        redact_fields: Arc::new(config.redact_fields.clone()),
        log_bodies,
    };

    // ── JWT validation (optional — enabled when JWT_SECRET is set) ───────
    let jwt_validator: Option<Arc<JwtValidator<StandardClaims>>> =
        (!config.auth.secret.is_empty()).then(|| {
            let mut v = JwtValidator::<StandardClaims>::hs256(config.auth.secret.as_bytes())
                .with_audience([config.auth.audience.as_str()])
                .with_leeway(config.auth.leeway_secs);
            if !config.auth.issuer.is_empty() {
                v = v.with_issuer([config.auth.issuer.as_str()]);
            }
            tracing::info!(
                issuer = %config.auth.issuer,
                audience = %config.auth.audience,
                "JWT auth enabled"
            );
            Arc::new(v)
        });
    if jwt_validator.is_none() {
        tracing::warn!("JWT_SECRET not set — admin endpoints are UNPROTECTED");
    }

    // ── Order batch aggregation (gRPC fan-out) ───────────────────────────
    let agg_state = AggregationState::new(order_channel);

    // ── OpenAPI (base + downstream order-service merged) ─────────────────
    let base_spec = serde_json::to_value(AdminApiDoc::openapi())
        .unwrap_or_else(|_| serde_json::json!({}));
    let downstream_spec_url = format!(
        "{}/api-docs/openapi.json",
        config.services.order_service
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
        .route("/admin/catalog/summary", get(aggregation::get_catalog_summary))
        .with_state(product_client);

    // Order batch aggregation (gRPC fan-out — registered before order_routes so /batch wins)
    let aggregation_routes = Router::new()
        .route("/admin/orders/batch", post(batch_get_orders))
        .with_state(agg_state);

    // Order CRUD (REST → gRPC pass-through)
    let order_routes = Router::new()
        .route("/admin/orders",             post(orders::create_order).get(orders::list_orders))
        .route("/admin/orders/{id}",         get(orders::get_order))
        .route("/admin/orders/{id}/confirm", put(orders::confirm_order))
        .route("/admin/orders/{id}/cancel",  put(orders::cancel_order))
        .with_state(order_client);

    // Group /admin/* routes and guard them with JWT auth when configured.
    let mut admin_routes = Router::new()
        .merge(product_routes)
        .merge(aggregation_routes)
        .merge(order_routes);
    if let Some(validator) = jwt_validator.clone() {
        admin_routes = admin_routes.layer(axum_mw::from_fn_with_state(
            validator,
            jwt_auth_layer::<StandardClaims>,
        ));
    }

    let app = Router::new()
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
        .layer(TimeoutLayer::with_status_code(axum::http::StatusCode::REQUEST_TIMEOUT, config.request_timeout));

    // ── Serve ────────────────────────────────────────────────────────────
    let addr: std::net::SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .expect("invalid ADMIN_BFF_HOST/ADMIN_BFF_PORT");

    tracing::info!(
        addr = %addr,
        request_timeout = ?config.request_timeout,
        "Admin BFF listening | docs at http://0.0.0.0:{}/scalar",
        config.port,
    );

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(ddd_bff::edge::shutdown::wait_for_shutdown_signal())
    .await
    .expect("Admin BFF server error");

    tracing::info!("Admin BFF shutdown complete");
}

/// Declarative table of every BFF endpoint that should appear in the
/// merged OpenAPI document with `x-bff-kind` metadata. Adding a new
/// pass-through, aggregation, or streaming endpoint is a one-line
/// addition here — no per-handler `#[utoipa::path]` boilerplate and no
/// hand-rolled JSON injection required.
///
/// Schemas referenced below must already exist in `components.schemas`,
/// either because they derive `utoipa::ToSchema` (and are registered on
/// `AdminApiDoc` or via proto build-time derives) or because
/// `merged_openapi` folded them in from a downstream service spec.
const API_ROUTES: &[ApiRoute] = &[
    // ── Pass-through (REST → gRPC) ─────────────────────────────────────
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "product",
            grpc_method: "product.v1.ProductService/CreateProduct",
        },
        method: "POST",
        path: "/admin/products",
        operation_id: "create_product",
        summary: "Create a product",
        tag: "Products",
        params: &[],
        request_body: Some(SchemaRef {
            name: "CreateProductRequest",
            content_type: "application/json",
        }),
        responses: &[
            ResponseSpec {
                status: 201,
                description: "Created",
                schema: Some(SchemaRef {
                    name: "CreateProductResponse",
                    content_type: "application/json",
                }),
            },
            ResponseSpec {
                status: 400,
                description: "Validation error",
                schema: Some(SchemaRef {
                    name: "ProblemDetail",
                    content_type: "application/problem+json",
                }),
            },
        ],
    },
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "product",
            grpc_method: "product.v1.ProductService/GetProduct",
        },
        method: "GET",
        path: "/admin/products/{id}",
        operation_id: "get_product",
        summary: "Fetch a product by id",
        tag: "Products",
        params: &[Param {
            name: "id",
            location: "path",
            required: true,
            schema_type: "string",
            description: "Product id (UUID)",
        }],
        request_body: None,
        responses: &[
            ResponseSpec {
                status: 200,
                description: "Found",
                schema: Some(SchemaRef {
                    name: "GetProductResponse",
                    content_type: "application/json",
                }),
            },
            ResponseSpec {
                status: 404,
                description: "Not found",
                schema: Some(SchemaRef {
                    name: "ProblemDetail",
                    content_type: "application/problem+json",
                }),
            },
        ],
    },
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "product",
            grpc_method: "product.v1.ProductService/ListProducts",
        },
        method: "GET",
        path: "/admin/products",
        operation_id: "list_products",
        summary: "List products with pagination",
        tag: "Products",
        params: &[
            Param {
                name: "page",
                location: "query",
                required: false,
                schema_type: "integer",
                description: "Page number (default: 1)",
            },
            Param {
                name: "per_page",
                location: "query",
                required: false,
                schema_type: "integer",
                description: "Items per page (default: 20)",
            },
        ],
        request_body: None,
        responses: &[ResponseSpec {
            status: 200,
            description: "Page of products",
            schema: Some(SchemaRef {
                name: "ListProductsResponse",
                content_type: "application/json",
            }),
        }],
    },
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "product",
            grpc_method: "product.v1.ProductService/UpdateStock",
        },
        method: "PUT",
        path: "/admin/products/{id}/stock",
        operation_id: "update_stock",
        summary: "Update a product's stock level",
        tag: "Products",
        params: &[Param {
            name: "id",
            location: "path",
            required: true,
            schema_type: "string",
            description: "Product id (UUID)",
        }],
        request_body: Some(SchemaRef {
            name: "UpdateStockRequest",
            content_type: "application/json",
        }),
        responses: &[ResponseSpec {
            status: 200,
            description: "Updated",
            schema: None,
        }],
    },
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "product",
            grpc_method: "product.v1.ProductService/DeactivateProduct",
        },
        method: "PUT",
        path: "/admin/products/{id}/deactivate",
        operation_id: "deactivate_product",
        summary: "Deactivate a product",
        tag: "Products",
        params: &[Param {
            name: "id",
            location: "path",
            required: true,
            schema_type: "string",
            description: "Product id (UUID)",
        }],
        request_body: None,
        responses: &[ResponseSpec {
            status: 200,
            description: "Deactivated",
            schema: None,
        }],
    },
    // ── Aggregation (multi-call fan-out) ───────────────────────────────
    ApiRoute {
        kind: RouteKind::Aggregation,
        method: "GET",
        path: "/admin/catalog/summary",
        operation_id: "get_catalog_summary",
        summary: "Aggregate product counts and recent items",
        tag: "Aggregation",
        params: &[],
        request_body: None,
        responses: &[ResponseSpec {
            status: 200,
            description: "Aggregated catalog summary",
            schema: None,
        }],
    },
    ApiRoute {
        kind: RouteKind::Aggregation,
        method: "POST",
        path: "/admin/orders/batch",
        operation_id: "batch_get_orders",
        summary: "Fetch many orders in parallel (partial-failure tolerant)",
        tag: "Aggregation",
        params: &[],
        request_body: Some(SchemaRef {
            name: "BatchRequest",
            content_type: "application/json",
        }),
        responses: &[
            ResponseSpec {
                status: 200,
                description: "Per-id results",
                schema: Some(SchemaRef {
                    name: "BatchResponse",
                    content_type: "application/json",
                }),
            },
            ResponseSpec {
                status: 400,
                description: "Too many ids",
                schema: Some(SchemaRef {
                    name: "ProblemDetail",
                    content_type: "application/problem+json",
                }),
            },
        ],
    },
    // ── Orders (REST → gRPC pass-through) ─────────────────────────────
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "order",
            grpc_method: "order.v1.OrderService/CreateOrder",
        },
        method: "POST",
        path: "/admin/orders",
        operation_id: "create_order",
        summary: "Place a new order",
        tag: "Orders",
        params: &[],
        request_body: Some(SchemaRef {
            name: "CreateOrderRequest",
            content_type: "application/json",
        }),
        responses: &[
            ResponseSpec {
                status: 201,
                description: "Created",
                schema: Some(SchemaRef {
                    name: "CreateOrderResponse",
                    content_type: "application/json",
                }),
            },
            ResponseSpec {
                status: 400,
                description: "Validation error",
                schema: Some(SchemaRef {
                    name: "ProblemDetail",
                    content_type: "application/problem+json",
                }),
            },
        ],
    },
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "order",
            grpc_method: "order.v1.OrderService/GetOrder",
        },
        method: "GET",
        path: "/admin/orders/{id}",
        operation_id: "get_order",
        summary: "Fetch an order by id",
        tag: "Orders",
        params: &[Param {
            name: "id",
            location: "path",
            required: true,
            schema_type: "string",
            description: "Order id (UUID)",
        }],
        request_body: None,
        responses: &[
            ResponseSpec {
                status: 200,
                description: "Found",
                schema: Some(SchemaRef {
                    name: "GetOrderResponse",
                    content_type: "application/json",
                }),
            },
            ResponseSpec {
                status: 404,
                description: "Not found",
                schema: Some(SchemaRef {
                    name: "ProblemDetail",
                    content_type: "application/problem+json",
                }),
            },
        ],
    },
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "order",
            grpc_method: "order.v1.OrderService/ListOrders",
        },
        method: "GET",
        path: "/admin/orders",
        operation_id: "list_orders",
        summary: "List orders with pagination",
        tag: "Orders",
        params: &[
            Param {
                name: "page",
                location: "query",
                required: false,
                schema_type: "integer",
                description: "Page number (default: 1)",
            },
            Param {
                name: "per_page",
                location: "query",
                required: false,
                schema_type: "integer",
                description: "Items per page (default: 20)",
            },
        ],
        request_body: None,
        responses: &[ResponseSpec {
            status: 200,
            description: "Page of orders",
            schema: Some(SchemaRef {
                name: "ListOrdersResponse",
                content_type: "application/json",
            }),
        }],
    },
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "order",
            grpc_method: "order.v1.OrderService/ConfirmOrder",
        },
        method: "PUT",
        path: "/admin/orders/{id}/confirm",
        operation_id: "confirm_order",
        summary: "Confirm a pending order",
        tag: "Orders",
        params: &[Param {
            name: "id",
            location: "path",
            required: true,
            schema_type: "string",
            description: "Order id (UUID)",
        }],
        request_body: None,
        responses: &[ResponseSpec {
            status: 200,
            description: "Confirmed",
            schema: None,
        }],
    },
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "order",
            grpc_method: "order.v1.OrderService/CancelOrder",
        },
        method: "PUT",
        path: "/admin/orders/{id}/cancel",
        operation_id: "cancel_order",
        summary: "Cancel an order",
        tag: "Orders",
        params: &[Param {
            name: "id",
            location: "path",
            required: true,
            schema_type: "string",
            description: "Order id (UUID)",
        }],
        request_body: Some(SchemaRef {
            name: "CancelOrderRequest",
            content_type: "application/json",
        }),
        responses: &[ResponseSpec {
            status: 200,
            description: "Cancelled",
            schema: None,
        }],
    },
    // ── Image upload (presigned URL flow) ─────────────────────────────
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "product",
            grpc_method: "product.v1.ProductService/RequestImageUploadUrl",
        },
        method: "POST",
        path: "/admin/products/{id}/image-upload-url",
        operation_id: "request_image_upload_url",
        summary: "Get a presigned PUT URL for uploading a product image",
        tag: "Products",
        params: &[Param {
            name: "id",
            location: "path",
            required: true,
            schema_type: "string",
            description: "Product id (UUID)",
        }],
        request_body: Some(SchemaRef {
            name: "ImageUploadUrlBody",
            content_type: "application/json",
        }),
        responses: &[
            ResponseSpec {
                status: 200,
                description: "Presigned URL",
                schema: Some(SchemaRef {
                    name: "RequestImageUploadUrlResponse",
                    content_type: "application/json",
                }),
            },
            ResponseSpec {
                status: 404,
                description: "Product not found",
                schema: Some(SchemaRef {
                    name: "ProblemDetail",
                    content_type: "application/problem+json",
                }),
            },
        ],
    },
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "product",
            grpc_method: "product.v1.ProductService/ConfirmImageUpload",
        },
        method: "POST",
        path: "/admin/products/{id}/confirm-image",
        operation_id: "confirm_image_upload",
        summary: "Confirm a product image was successfully uploaded to blob storage",
        tag: "Products",
        params: &[Param {
            name: "id",
            location: "path",
            required: true,
            schema_type: "string",
            description: "Product id (UUID)",
        }],
        request_body: Some(SchemaRef {
            name: "ConfirmImageBody",
            content_type: "application/json",
        }),
        responses: &[
            ResponseSpec {
                status: 204,
                description: "Confirmed",
                schema: None,
            },
            ResponseSpec {
                status: 404,
                description: "Product not found",
                schema: Some(SchemaRef {
                    name: "ProblemDetail",
                    content_type: "application/problem+json",
                }),
            },
            ResponseSpec {
                status: 409,
                description: "Product is inactive",
                schema: Some(SchemaRef {
                    name: "ProblemDetail",
                    content_type: "application/problem+json",
                }),
            },
        ],
    },
];
