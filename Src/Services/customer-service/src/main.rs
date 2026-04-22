//! Customer Service — gRPC server (gRPC + gRPC-Web on port 50055).
//!
//! Wires the Postgres repositories, S3 blob storage, and mediator into a
//! single tonic server. DB bootstrap: create the `customer` schema if missing,
//! run sqlx migrations, then reconnect with `search_path=customer,public`.

use std::net::SocketAddr;
use std::sync::Arc;

use ddd_application::Mediator;
use ddd_infrastructure::storage::{S3BlobStorage, S3Config};
use ddd_infrastructure::{create_pool, run_migrations_from_path};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use customer_service::api::grpc::CustomerGrpcService;
use customer_service::application::deps::AppDeps;
use customer_service::infrastructure::db::repositories::{
    PgCustomerProfileRepository, PgCustomerRepository, PgWishlistItemRepository,
};
use customer_service::infrastructure::db::seeder;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Customer Service");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/modernstores".into());

    // Ensure the `customer` schema exists before sqlx migrations run.
    let init_pool = create_pool(&database_url)
        .await
        .expect("failed to connect to database for schema bootstrap");
    init_pool
        .execute(Statement::from_string(
            DatabaseBackend::Postgres,
            "CREATE SCHEMA IF NOT EXISTS customer".to_owned(),
        ))
        .await
        .expect("failed to create `customer` schema");
    drop(init_pool);

    run_migrations_from_path(&database_url, "./migrations")
        .await
        .expect("failed to run customer-service migrations");

    // Connect with search_path pinned to `customer,public`.
    let url_with_path = if database_url.contains('?') {
        format!("{database_url}&options=-c%20search_path%3Dcustomer,public")
    } else {
        format!("{database_url}?options=-c%20search_path%3Dcustomer,public")
    };
    let db = Arc::new(
        create_pool(&url_with_path)
            .await
            .expect("failed to connect to database"),
    );

    seeder::run_seeder(&db).await;

    // ── Blob storage ────────────────────────────────────────────────────────
    let storage = Arc::new(
        S3BlobStorage::connect(s3_config_from_env())
            .await
            .expect("failed to build S3 client"),
    );
    let blob_bucket = std::env::var("CUSTOMER_BLOB_BUCKET")
        .unwrap_or_else(|_| "customer-assets".into());
    let presign_ttl_secs: u64 = parse_env("CUSTOMER_PRESIGN_TTL_SECS", 900);

    let deps = AppDeps {
        customer_repo: Arc::new(PgCustomerRepository(db.clone())),
        profile_repo: Arc::new(PgCustomerProfileRepository(db.clone())),
        wishlist_repo: Arc::new(PgWishlistItemRepository(db.clone())),
        blob_storage: storage,
        blob_bucket,
        presign_ttl_secs,
    };
    let mediator = Arc::new(Mediator::from_inventory(&deps));

    let grpc_service = CustomerGrpcService::new(mediator);

    let addr: SocketAddr = ([0, 0, 0, 0], 50055).into();
    tracing::info!("gRPC + gRPC-Web on 0.0.0.0:50055");

    if let Err(e) = Server::builder()
        .accept_http1(true)
        .layer(GrpcWebLayer::new())
        .add_service(grpc_service.into_server())
        .serve_with_shutdown(addr, shutdown_signal())
        .await
    {
        tracing::error!(error = %e, "gRPC server exited with error");
    }
}

fn s3_config_from_env() -> S3Config {
    S3Config {
        endpoint: std::env::var("S3_ENDPOINT").ok(),
        region: std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".into()),
        force_path_style: std::env::var("S3_FORCE_PATH_STYLE")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false),
        access_key_id: std::env::var("AWS_ACCESS_KEY_ID").ok(),
        secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").ok(),
        session_token: std::env::var("AWS_SESSION_TOKEN").ok(),
    }
}

fn parse_env<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => tracing::info!("received SIGINT"),
        () = terminate => tracing::info!("received SIGTERM"),
    }
}
