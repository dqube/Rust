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

use ddd_bff::clients::GrpcClientPool;
use ddd_infrastructure::cache::RedisCache;
use ddd_shared_kernel::jwt::{JwtValidator, StandardClaims};
use ddd_shared_kernel::Cache;
use tracing_subscriber::{
    layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

use admin_bff::application::config::AdminBffConfig;

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

    // ── gRPC channels (product + order + shared) via ddd-bff resilient pool ─
    let pool = GrpcClientPool::from_services(
        [
            ("product", config.services.product_service.as_str()),
            ("order",   config.services.order_service.as_str()),
            ("shared",  config.services.shared_service.as_str()),
        ],
        &config.resilience,
    )
    .expect("failed to build gRPC client pool");

    tracing::info!(url = %config.services.product_service, "connected to product-service (lazy)");
    tracing::info!(url = %config.services.order_service,   "connected to order-service (lazy)");
    tracing::info!(url = %config.services.shared_service,  "connected to shared-service (lazy)");

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

    // ── Cache (optional — enabled when REDIS_URL is set) ─────────────────
    let cache: Option<Arc<dyn Cache>> = if config.cache.redis_url.is_empty() {
        tracing::warn!("REDIS_URL not set — read-through cache disabled");
        None
    } else {
        match RedisCache::connect(&config.cache.redis_url, config.cache.key_prefix.clone()).await
        {
            Ok(c) => {
                tracing::info!(
                    url = %config.cache.redis_url,
                    prefix = %config.cache.key_prefix,
                    "Redis cache enabled"
                );
                Some(Arc::new(c) as Arc<dyn Cache>)
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to connect to Redis — cache disabled");
                None
            }
        }
    };

    // ── Unified AppState ──────────────────────────────────────────
    let state = admin_bff::application::state::AppState::new(config.clone(), pool, jwt_validator, cache);

    // ── Build Router ─────────────────────────────────────────────────────
    let app = admin_bff::api::router::build_router(state).await;

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
