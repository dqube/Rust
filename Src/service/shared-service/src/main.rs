//! Shared Service — gRPC server (gRPC + gRPC-Web on port 50053).

use std::net::SocketAddr;
use std::sync::Arc;

use ddd_application::Mediator;
use ddd_infrastructure::{create_pool, run_migrations_from_path};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use shared_service::api::grpc::SharedGrpcService;
use shared_service::application::deps::AppDeps;
use shared_service::infrastructure::db::repositories::{
    PgCityRepository, PgCountryRepository, PgCurrencyRepository, PgPincodeRepository,
    PgStateRepository,
};
use shared_service::infrastructure::db::seeder;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Shared Service");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/modernstores".into());

    // Ensure the `shared` schema exists before sqlx migrations run.
    let init_pool = create_pool(&database_url)
        .await
        .expect("failed to connect to database for schema bootstrap");
    init_pool
        .execute(Statement::from_string(
            DatabaseBackend::Postgres,
            "CREATE SCHEMA IF NOT EXISTS shared".to_owned(),
        ))
        .await
        .expect("failed to create `shared` schema");
    drop(init_pool);

    // Run sqlx file-based migrations.
    run_migrations_from_path(&database_url, "./migrations")
        .await
        .expect("failed to run shared-service migrations");

    // Connect with search_path pinned to `shared,public`.
    let url_with_path = if database_url.contains('?') {
        format!("{database_url}&options=-c%20search_path%3Dshared,public")
    } else {
        format!("{database_url}?options=-c%20search_path%3Dshared,public")
    };
    let db = Arc::new(
        create_pool(&url_with_path)
            .await
            .expect("failed to connect to database"),
    );

    // Seed reference data (idempotent).
    seeder::run_seeder(&db).await;

    let deps = AppDeps {
        currency_repo: Arc::new(PgCurrencyRepository(db.clone())),
        country_repo: Arc::new(PgCountryRepository(db.clone())),
        state_repo: Arc::new(PgStateRepository(db.clone())),
        city_repo: Arc::new(PgCityRepository(db.clone())),
        pincode_repo: Arc::new(PgPincodeRepository(db.clone())),
    };
    let mediator = Arc::new(Mediator::from_inventory(&deps));

    let grpc_service = SharedGrpcService::new(mediator);

    let addr: SocketAddr = ([0, 0, 0, 0], 50053).into();
    tracing::info!("gRPC + gRPC-Web on 0.0.0.0:50053");

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
