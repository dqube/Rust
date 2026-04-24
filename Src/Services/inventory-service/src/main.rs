use std::net::SocketAddr;
use std::sync::Arc;

use ddd_application::Mediator;
use ddd_infrastructure::create_pool;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use sea_orm_migration::MigratorTrait;
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use inventory_service::api::grpc::InventoryGrpcService;
use inventory_service::application::deps::AppDeps;
use inventory_service::infrastructure::db::{
    migrations::Migrator,
    repositories::{PgInventoryItemRepository, PgStockMovementRepository},
    seeder,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting inventory-service");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/modernstores".into());

    let init_pool = create_pool(&database_url)
        .await
        .expect("failed to connect to database for schema bootstrap");
    init_pool
        .execute(Statement::from_string(
            DatabaseBackend::Postgres,
            "CREATE SCHEMA IF NOT EXISTS inventory".to_owned(),
        ))
        .await
        .expect("failed to create inventory schema");
    Migrator::up(&init_pool, None)
        .await
        .expect("failed to run inventory-service migrations");
    drop(init_pool);

    let url_with_path = if database_url.contains('?') {
        format!("{database_url}&options=-c%20search_path%3Dinventory,public")
    } else {
        format!("{database_url}?options=-c%20search_path%3Dinventory,public")
    };
    let db = Arc::new(
        create_pool(&url_with_path)
            .await
            .expect("failed to connect to inventory database"),
    );

    seeder::run_seeder(&db).await;

    let deps = AppDeps {
        inventory_repo: Arc::new(PgInventoryItemRepository(db.clone())),
        stock_movement_repo: Arc::new(PgStockMovementRepository(db.clone())),
    };
    let mediator = Arc::new(Mediator::from_inventory(&deps));

    let grpc_service = InventoryGrpcService::new(mediator);

    let addr: SocketAddr = ([0, 0, 0, 0], 50056).into();
    tracing::info!("gRPC + gRPC-Web on 0.0.0.0:50056");

    if let Err(error) = Server::builder()
        .accept_http1(true)
        .layer(GrpcWebLayer::new())
        .add_service(grpc_service.into_server())
        .serve(addr)
        .await
    {
        tracing::error!(%error, "gRPC server exited with error");
    }
}