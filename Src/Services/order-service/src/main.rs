//! Order Service — gRPC server (gRPC + gRPC-Web on port 8080).

use std::net::SocketAddr;
use std::sync::Arc;

use ddd_application::Mediator;
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use order_service::api::grpc::OrderGrpcService;
use order_service::application::deps::AppDeps;
use order_service::infrastructure::in_memory_repo::InMemoryOrderRepository;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Order Service");

    let order_repo = Arc::new(InMemoryOrderRepository::new());
    let deps = AppDeps {
        order_repo: order_repo.clone(),
    };
    let mediator = Arc::new(Mediator::from_inventory(&deps));

    let grpc_service = OrderGrpcService::new(mediator);

    let addr: SocketAddr = ([0, 0, 0, 0], 8080).into();
    tracing::info!("gRPC + gRPC-Web on 0.0.0.0:8080");

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
