//! Product Service — gRPC server (gRPC + gRPC-Web on port 50052).

use std::net::SocketAddr;
use std::sync::Arc;

use ddd_application::Mediator;
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use product_service::api::grpc::ProductGrpcService;
use product_service::application::deps::AppDeps;
use product_service::infrastructure::in_memory_repo::InMemoryProductRepository;
use product_service::infrastructure::local_storage_stub::LocalStorageStub;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Product Service");

    let product_repo = Arc::new(InMemoryProductRepository::new());
    let storage = Arc::new(LocalStorageStub::new());
    let deps = AppDeps {
        product_repo: product_repo.clone(),
        storage: storage.clone(),
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
