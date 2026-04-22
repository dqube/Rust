//! Product Service — gRPC server (gRPC + gRPC-Web on port 50052).

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use ddd_application::Mediator;
use ddd_infrastructure::storage::{S3BlobStorage, S3Config};
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use product_service::api::grpc::ProductGrpcService;
use product_service::application::deps::AppDeps;
use product_service::infrastructure::in_memory_repo::InMemoryProductRepository;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Product Service");

    let product_repo = Arc::new(InMemoryProductRepository::new());
    let storage = Arc::new(
        S3BlobStorage::connect(s3_config_from_env())
            .await
            .expect("failed to build S3 client"),
    );
    let image_bucket = std::env::var("PRODUCT_IMAGE_BUCKET")
        .unwrap_or_else(|_| "product-images".into());
    let presign_ttl = Duration::from_secs(
        std::env::var("PRODUCT_PRESIGN_TTL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(900),
    );
    let deps = AppDeps {
        product_repo: product_repo.clone(),
        storage: storage.clone(),
        image_bucket,
        presign_ttl,
    };
    let mediator = Arc::new(Mediator::from_inventory(&deps));

    let grpc_service = ProductGrpcService::new(mediator);

    let addr: SocketAddr = ([0, 0, 0, 0], 50052).into();
    tracing::info!("gRPC + gRPC-Web on 0.0.0.0:50052");

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

/// Build an [`S3Config`] from `S3_*` env vars.
///
/// All fields are optional; unset values fall back to the AWS default
/// credential / region chain. Local-dev workflows typically set
/// `S3_ENDPOINT=http://localhost:9000`, `S3_FORCE_PATH_STYLE=true`, plus
/// MinIO credentials.
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
