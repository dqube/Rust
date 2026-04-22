//! Supplier Service — gRPC server (gRPC + gRPC-Web on port 50057).

use std::net::SocketAddr;
use std::sync::Arc;

use ddd_application::Mediator;
use ddd_infrastructure::storage::{S3BlobStorage, S3Config};
use ddd_infrastructure::{create_pool, run_migrations_from_path};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use supplier_service::api::grpc::SupplierGrpcService;
use supplier_service::application::deps::AppDeps;
use supplier_service::infrastructure::db::repositories::*;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Supplier Service");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/modernstores".into());

    let init_pool = create_pool(&database_url)
        .await
        .expect("failed to connect to database for schema bootstrap");
    init_pool
        .execute(Statement::from_string(
            DatabaseBackend::Postgres,
            "CREATE SCHEMA IF NOT EXISTS supplier".to_owned(),
        ))
        .await
        .expect("failed to create `supplier` schema");
    drop(init_pool);

    run_migrations_from_path(&database_url, "./migrations")
        .await
        .expect("failed to run supplier-service migrations");

    let url_with_path = if database_url.contains('?') {
        format!("{database_url}&options=-c%20search_path%3Dsupplier,public")
    } else {
        format!("{database_url}?options=-c%20search_path%3Dsupplier,public")
    };
    let db = Arc::new(
        create_pool(&url_with_path)
            .await
            .expect("failed to connect to database"),
    );

    let storage = Arc::new(
        S3BlobStorage::connect(s3_config_from_env())
            .await
            .expect("failed to build S3 client"),
    );
    let blob_bucket      = std::env::var("SUPPLIER_BLOB_BUCKET")
        .unwrap_or_else(|_| "supplier-assets".into());
    let presign_ttl_secs: u64 = parse_env("SUPPLIER_PRESIGN_TTL_SECS", 900);

    let deps = AppDeps {
        supplier_repo:    Arc::new(PgSupplierRepository(db.clone())),
        address_repo:     Arc::new(PgSupplierAddressRepository(db.clone())),
        contact_repo:     Arc::new(PgSupplierContactRepository(db.clone())),
        document_repo:    Arc::new(PgSupplierDocumentRepository(db.clone())),
        product_repo:     Arc::new(PgSupplierProductRepository(db.clone())),
        order_repo:       Arc::new(PgPurchaseOrderRepository(db.clone())),
        blob_storage:     storage,
        blob_bucket,
        presign_ttl_secs,
    };
    let mediator     = Arc::new(Mediator::from_inventory(&deps));
    let grpc_service = SupplierGrpcService::new(mediator);

    let addr: SocketAddr = ([0, 0, 0, 0], 50057).into();
    tracing::info!("gRPC + gRPC-Web on 0.0.0.0:50057");

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
        endpoint:          std::env::var("S3_ENDPOINT").ok(),
        region:            std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".into()),
        force_path_style:  std::env::var("S3_FORCE_PATH_STYLE")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false),
        access_key_id:     std::env::var("AWS_ACCESS_KEY_ID").ok(),
        secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").ok(),
        session_token:     std::env::var("AWS_SESSION_TOKEN").ok(),
    }
}

fn parse_env<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
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
        () = ctrl_c    => tracing::info!("received SIGINT"),
        () = terminate => tracing::info!("received SIGTERM"),
    }
}
