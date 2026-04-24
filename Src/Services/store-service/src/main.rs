#![allow(dead_code)]

use std::net::SocketAddr;
use std::sync::Arc;

use ddd_application::Mediator;
use ddd_infrastructure::db::SeaOrmOutboxRepository;
use ddd_infrastructure::messaging::JetStreamPublisher;
use ddd_infrastructure::storage::{S3BlobStorage, S3Config};
use ddd_infrastructure::{create_pool, SeaOrmDeadLetterRepository};
use ddd_shared_kernel::{LogDeadLetterAlert, OutboxRelay};
use futures::StreamExt;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use sea_orm_migration::MigratorTrait;
use tonic::transport::Server;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod api;
mod application;
mod domain;
mod infrastructure;

use api::grpc::store_grpc::StoreGrpcService;
use application::deps::AppDeps;
use application::handlers::integration_handlers::handle_employee_store_assigned;
use application::integration_events::EmployeeStoreAssignedIntegrationEvent;
use infrastructure::db::{
    migrations::Migrator,
    repositories::{PgRegisterRepository, PgStoreRepository},
};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting store-service");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/modernstores".into());

    // Bootstrap schema
    let init_pool = create_pool(&database_url).await.expect("failed to connect for schema init");
    init_pool.execute(Statement::from_string(
        DatabaseBackend::Postgres,
        "CREATE SCHEMA IF NOT EXISTS store".to_owned(),
    )).await.expect("failed to create store schema");

    // Run SeaORM migrations
    Migrator::up(&init_pool, None).await.expect("migrations failed");
    tracing::info!("Migrations applied.");
    drop(init_pool);

    // Main connection pool with search_path
    let url_with_path = if database_url.contains('?') {
        format!("{database_url}&options=-c%20search_path%3Dstore,public")
    } else {
        format!("{database_url}?options=-c%20search_path%3Dstore,public")
    };
    let db = Arc::new(create_pool(&url_with_path).await.expect("failed to connect to database"));

    // Repositories
    let store_repo    = Arc::new(PgStoreRepository(db.clone()))    as Arc<dyn domain::repositories::StoreRepository>;
    let register_repo = Arc::new(PgRegisterRepository(db.clone())) as Arc<dyn domain::repositories::RegisterRepository>;

    // Outbox
    let outbox_pool = create_pool(&database_url).await.expect("outbox pool");
    let outbox_repo = Arc::new(SeaOrmOutboxRepository::new(outbox_pool))
        as Arc<dyn ddd_shared_kernel::OutboxRepository>;

    // Blob storage
    let blob_storage = Arc::new(
        S3BlobStorage::connect(s3_config_from_env()).await.expect("failed to build S3 client"),
    ) as Arc<dyn ddd_shared_kernel::BlobStorage>;
    let blob_bucket = std::env::var("STORE_BLOB_BUCKET").unwrap_or_else(|_| "store-assets".into());

    let deps = AppDeps {
        store_repo:    store_repo.clone(),
        register_repo: register_repo.clone(),
        outbox:        outbox_repo.clone(),
        blob_storage,
        blob_bucket,
    };
    let mediator = Arc::new(Mediator::from_inventory(&deps));

    // NATS
    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".into());
    let nats_client = match async_nats::connect(&nats_url).await {
        Ok(c)  => { tracing::info!(url = %nats_url, "NATS connected"); Some(c) }
        Err(e) => { tracing::warn!(error = %e, "NATS unavailable — subscriptions disabled"); None }
    };

    if let Some(ref client) = nats_client {
        let js = async_nats::jetstream::new(client.clone());

        // Outbox relay
        let relay_db  = create_pool(&database_url).await.expect("outbox relay pool");
        let relay_out = Arc::new(SeaOrmOutboxRepository::new(relay_db.clone()));
        let relay_dlq = Arc::new(SeaOrmDeadLetterRepository::new(relay_db));
        match JetStreamPublisher::connect(&nats_url, "store").await {
            Ok(publisher) => {
                let relay = Arc::new(OutboxRelay::new(
                    relay_out, Arc::new(publisher), relay_dlq,
                    Arc::new(LogDeadLetterAlert), 50, 1000, 5,
                ));
                tokio::spawn(async move { relay.run().await });
                tracing::info!("OutboxRelay started (domain=store)");
            }
            Err(e) => tracing::warn!(error = %e, "JetStreamPublisher failed — relay disabled"),
        }

        // EmployeeStoreAssigned subscription
        let js2 = js.clone();
        tokio::spawn(async move {
            let stream_cfg = async_nats::jetstream::stream::Config {
                name:     EmployeeStoreAssignedIntegrationEvent::STREAM.to_string(),
                subjects: vec![format!("{}.*.*.*", EmployeeStoreAssignedIntegrationEvent::STREAM.to_lowercase())],
                ..Default::default()
            };
            let nats_stream = match js2.get_or_create_stream(stream_cfg).await {
                Ok(s)  => s,
                Err(e) => { tracing::error!(error = %e, "failed to get/create EMPLOYEE stream"); return; }
            };
            let consumer_cfg = async_nats::jetstream::consumer::pull::Config {
                durable_name:   Some(EmployeeStoreAssignedIntegrationEvent::CONSUMER.to_string()),
                filter_subject: EmployeeStoreAssignedIntegrationEvent::TOPIC.to_string(),
                ..Default::default()
            };
            let mut msg_stream = match nats_stream.create_consumer(consumer_cfg).await {
                Ok(c)  => match c.messages().await {
                    Ok(m)  => m,
                    Err(e) => { tracing::error!(error = %e, "failed to get message stream"); return; }
                },
                Err(e) => { tracing::error!(error = %e, "failed to create consumer"); return; }
            };

            tracing::info!(subject = EmployeeStoreAssignedIntegrationEvent::TOPIC, "NATS subscription active");

            while let Some(msg_result) = msg_stream.next().await {
                match msg_result {
                    Ok(msg) => {
                        match serde_json::from_slice::<EmployeeStoreAssignedIntegrationEvent>(&msg.payload) {
                            Ok(evt) => {
                                if let Err(e) = handle_employee_store_assigned(evt).await {
                                    tracing::error!(error = %e, "handle_employee_store_assigned error");
                                }
                            }
                            Err(e) => tracing::warn!(error = %e, "failed to deserialize EmployeeStoreAssigned"),
                        }
                        let _ = msg.ack().await;
                    }
                    Err(e) => tracing::warn!(error = %e, "NATS message error"),
                }
            }
        });
    }

    // gRPC server
    let grpc_port: u16 = std::env::var("GRPC_PORT")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(5131);
    let addr: SocketAddr = ([0, 0, 0, 0], grpc_port).into();
    tracing::info!("gRPC on 0.0.0.0:{grpc_port}");

    Server::builder()
        .add_service(StoreGrpcService::new(mediator).into_server())
        .serve_with_shutdown(addr, shutdown_signal())
        .await
        .expect("gRPC server failed");
}

fn s3_config_from_env() -> S3Config {
    S3Config {
        endpoint:          std::env::var("S3_ENDPOINT").ok()
            .or_else(|| std::env::var("MINIO_ENDPOINT").ok()),
        region:            std::env::var("S3_REGION")
            .or_else(|_| std::env::var("MINIO_REGION"))
            .unwrap_or_else(|_| "us-east-1".into()),
        force_path_style:  std::env::var("S3_FORCE_PATH_STYLE")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(true),
        access_key_id:     std::env::var("AWS_ACCESS_KEY_ID")
            .or_else(|_| std::env::var("MINIO_ACCESS_KEY")).ok(),
        secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY")
            .or_else(|_| std::env::var("MINIO_SECRET_KEY")).ok(),
        session_token:     std::env::var("AWS_SESSION_TOKEN").ok(),
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("failed to install Ctrl-C handler");
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
