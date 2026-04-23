//! Sales Service — gRPC server (gRPC + gRPC-Web on port 50060).

use std::net::SocketAddr;
use std::sync::Arc;

use ddd_application::Mediator;
use ddd_infrastructure::storage::{S3BlobStorage, S3Config};
use ddd_infrastructure::{create_pool, run_migrations_from_path};
use ddd_infrastructure::db::{SeaOrmDeadLetterRepository, SeaOrmOutboxRepository};
use ddd_infrastructure::messaging::JetStreamPublisher;
use ddd_shared_kernel::{LogDeadLetterAlert, OutboxRelay};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use sales_service::api::grpc::SalesGrpcService;
use sales_service::application::deps::AppDeps;
use sales_service::infrastructure::db::repositories::{
    PgOrderSagaRepository, PgReturnRepository, PgSaleRepository,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Sales Service");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/modernstores".into());

    // Bootstrap schema
    let init_pool = create_pool(&database_url)
        .await
        .expect("failed to connect to database for schema bootstrap");
    init_pool
        .execute(Statement::from_string(
            DatabaseBackend::Postgres,
            "CREATE SCHEMA IF NOT EXISTS sales".to_owned(),
        ))
        .await
        .expect("failed to create `sales` schema");
    drop(init_pool);

    // Run migrations
    run_migrations_from_path(&database_url, "./migrations")
        .await
        .expect("failed to run sales-service migrations");

    // Main connection pool with search_path
    let url_with_path = if database_url.contains('?') {
        format!("{database_url}&options=-c%20search_path%3Dsales,public")
    } else {
        format!("{database_url}?options=-c%20search_path%3Dsales,public")
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
    let blob_bucket = std::env::var("SALES_BLOB_BUCKET")
        .unwrap_or_else(|_| "sales-receipts".into());
    let presign_ttl_secs: u64 = parse_env("SALES_PRESIGN_TTL_SECS", 3600);

    // Repositories
    let sale_repo   = Arc::new(PgSaleRepository(db.clone()));
    let return_repo = Arc::new(PgReturnRepository(db.clone()));
    let saga_repo   = Arc::new(PgOrderSagaRepository(db.clone()));

    // Outbox uses public schema (ddd-infrastructure default table: outbox_messages)
    let outbox_db   = create_pool(&database_url).await.expect("outbox pool");
    let outbox_repo = Arc::new(SeaOrmOutboxRepository::new(outbox_db));

    let deps = AppDeps {
        sale_repo:        sale_repo.clone() as Arc<dyn sales_service::domain::repositories::SaleRepository>,
        return_repo:      return_repo.clone() as Arc<dyn sales_service::domain::repositories::ReturnRepository>,
        saga_repo:        saga_repo.clone() as Arc<dyn sales_service::domain::repositories::OrderSagaRepository>,
        outbox:           outbox_repo.clone() as Arc<dyn ddd_shared_kernel::OutboxRepository>,
        blob_storage:     storage.clone() as Arc<dyn ddd_shared_kernel::BlobStorage>,
        blob_bucket,
        presign_ttl_secs,
    };
    let mediator = Arc::new(Mediator::from_inventory(&deps));
    let grpc_service = SalesGrpcService::new(mediator);

    // ── NATS subscriptions for saga + integration events ─────────────────────
    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".into());
    let nats_client = match async_nats::connect(&nats_url).await {
        Ok(c) => {
            tracing::info!(url = %nats_url, "NATS connected");
            Some(c)
        }
        Err(e) => {
            tracing::warn!(error = %e, "NATS unavailable — subscriptions disabled");
            None
        }
    };

    if let Some(ref client) = nats_client {
        let js = async_nats::jetstream::new(client.clone());

        // ── Outbox relay — publishes pending outbox rows to NATS JetStream ─
        let outbox_relay_db = create_pool(&database_url).await.expect("outbox relay pool");
        let relay_outbox  = Arc::new(SeaOrmOutboxRepository::new(outbox_relay_db.clone()));
        let relay_dlq     = Arc::new(SeaOrmDeadLetterRepository::new(outbox_relay_db));
        match JetStreamPublisher::connect(&nats_url, "sales").await {
            Ok(publisher) => {
                let relay = Arc::new(OutboxRelay::new(
                    relay_outbox,
                    Arc::new(publisher),
                    relay_dlq,
                    Arc::new(LogDeadLetterAlert),
                    50,    // batch size
                    1000,  // poll interval ms
                    5,     // max attempts before DLQ
                ));
                tokio::spawn(async move { relay.run().await });
                tracing::info!("OutboxRelay started (JetStream, domain=sales)");
            }
            Err(e) => tracing::warn!(error = %e, "failed to create JetStreamPublisher — outbox relay disabled"),
        }

        // Helper type aliases
        type SaleRepo   = Arc<dyn sales_service::domain::repositories::SaleRepository>;
        type SagaRepo   = Arc<dyn sales_service::domain::repositories::OrderSagaRepository>;
        type Outbox     = Arc<dyn ddd_shared_kernel::OutboxRepository>;

        // Spawn subscriptions
        spawn_nats_subscription::<sales_service::application::integration_events::StockReservedIntegrationEvent>(
            js.clone(), "INVENTORY", "v1.inventory.stock.reserved", "sales-service-stock-reserved",
            sale_repo.clone() as SaleRepo, saga_repo.clone() as SagaRepo, outbox_repo.clone() as Outbox,
            |evt, sale_repo, saga_repo, outbox| {
                let sale_repo = sale_repo.clone();
                let saga_repo = saga_repo.clone();
                let outbox    = outbox.clone();
                Box::pin(async move {
                    if let Err(e) = sales_service::application::saga::handle_stock_reserved(evt, &saga_repo, &sale_repo, &outbox).await {
                        tracing::error!("handle_stock_reserved error: {e}");
                    }
                })
            },
        );

        spawn_nats_subscription::<sales_service::application::integration_events::StockReservationFailedIntegrationEvent>(
            js.clone(), "INVENTORY", "v1.inventory.stock.reservation-failed", "sales-service-stock-reservation-failed",
            sale_repo.clone() as SaleRepo, saga_repo.clone() as SagaRepo, outbox_repo.clone() as Outbox,
            |evt, sale_repo, saga_repo, outbox| {
                let sale_repo = sale_repo.clone();
                let saga_repo = saga_repo.clone();
                let outbox    = outbox.clone();
                Box::pin(async move {
                    if let Err(e) = sales_service::application::saga::handle_stock_reservation_failed(evt, &saga_repo, &sale_repo, &outbox).await {
                        tracing::error!("handle_stock_reservation_failed error: {e}");
                    }
                })
            },
        );

        spawn_nats_subscription::<sales_service::application::integration_events::PaymentCapturedIntegrationEvent>(
            js.clone(), "PAYMENT", "v1.payment.payment.captured", "sales-service-payment-captured",
            sale_repo.clone() as SaleRepo, saga_repo.clone() as SagaRepo, outbox_repo.clone() as Outbox,
            |evt, sale_repo, saga_repo, outbox| {
                let sale_repo = sale_repo.clone();
                let saga_repo = saga_repo.clone();
                let outbox    = outbox.clone();
                Box::pin(async move {
                    if let Err(e) = sales_service::application::saga::handle_payment_captured(evt, &saga_repo, &sale_repo, &outbox).await {
                        tracing::error!("handle_payment_captured error: {e}");
                    }
                })
            },
        );

        spawn_nats_subscription::<sales_service::application::integration_events::PaymentFailedIntegrationEvent>(
            js.clone(), "PAYMENT", "v1.payment.payment.failed", "sales-service-payment-failed",
            sale_repo.clone() as SaleRepo, saga_repo.clone() as SagaRepo, outbox_repo.clone() as Outbox,
            |evt, sale_repo, saga_repo, outbox| {
                let sale_repo = sale_repo.clone();
                let saga_repo = saga_repo.clone();
                let outbox    = outbox.clone();
                Box::pin(async move {
                    if let Err(e) = sales_service::application::saga::handle_payment_failed(evt, &saga_repo, &sale_repo, &outbox).await {
                        tracing::error!("handle_payment_failed error: {e}");
                    }
                })
            },
        );

        spawn_nats_subscription::<sales_service::application::integration_events::PaymentInitiationFailedIntegrationEvent>(
            js.clone(), "PAYMENT", "v1.payment.payment.initiation-failed", "sales-service-payment-initiation-failed",
            sale_repo.clone() as SaleRepo, saga_repo.clone() as SagaRepo, outbox_repo.clone() as Outbox,
            |evt, sale_repo, saga_repo, outbox| {
                let sale_repo = sale_repo.clone();
                let saga_repo = saga_repo.clone();
                let outbox    = outbox.clone();
                Box::pin(async move {
                    if let Err(e) = sales_service::application::saga::handle_payment_initiation_failed(evt, &saga_repo, &sale_repo, &outbox).await {
                        tracing::error!("handle_payment_initiation_failed error: {e}");
                    }
                })
            },
        );

        spawn_nats_subscription::<sales_service::application::integration_events::PaymentRefundedIntegrationEvent>(
            js.clone(), "PAYMENT", "v1.payment.payment.refunded", "sales-service-payment-refunded",
            sale_repo.clone() as SaleRepo, saga_repo.clone() as SagaRepo, outbox_repo.clone() as Outbox,
            |evt, sale_repo, saga_repo, outbox| {
                let sale_repo = sale_repo.clone();
                let saga_repo = saga_repo.clone();
                let outbox    = outbox.clone();
                Box::pin(async move {
                    if let Err(e) = sales_service::application::saga::handle_payment_refunded(evt, &saga_repo, &sale_repo, &outbox).await {
                        tracing::error!("handle_payment_refunded error: {e}");
                    }
                })
            },
        );

        spawn_nats_subscription::<sales_service::application::integration_events::PromotionAppliedIntegrationEvent>(
            js.clone(), "PROMOTION", "v1.promotion.promotion.applied", "sales-service-promotion-applied",
            sale_repo.clone() as SaleRepo, saga_repo.clone() as SagaRepo, outbox_repo.clone() as Outbox,
            |evt, sale_repo, saga_repo, outbox| {
                let sale_repo = sale_repo.clone();
                let saga_repo = saga_repo.clone();
                let outbox    = outbox.clone();
                Box::pin(async move {
                    if let Err(e) = sales_service::application::saga::handle_promotion_applied(evt, &saga_repo, &sale_repo, &outbox).await {
                        tracing::error!("handle_promotion_applied error: {e}");
                    }
                })
            },
        );

        spawn_nats_subscription::<sales_service::application::integration_events::CustomerDeletedIntegrationEvent>(
            js.clone(), "CUSTOMER", "v1.customer.customer.deleted", "sales-service-customer-deleted",
            sale_repo.clone() as SaleRepo, saga_repo.clone() as SagaRepo, outbox_repo.clone() as Outbox,
            |evt, sale_repo, saga_repo, outbox| {
                let sale_repo = sale_repo.clone();
                let saga_repo = saga_repo.clone();
                let outbox    = outbox.clone();
                Box::pin(async move {
                    if let Err(e) = sales_service::application::saga::handle_customer_deleted(evt, &saga_repo, &sale_repo, &outbox).await {
                        tracing::error!("handle_customer_deleted error: {e}");
                    }
                })
            },
        );
    }

    // ── gRPC server ───────────────────────────────────────────────────────────
    let addr: SocketAddr = ([0, 0, 0, 0], 50060).into();
    tracing::info!("gRPC + gRPC-Web on 0.0.0.0:50060");

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

fn spawn_nats_subscription<E>(
    js:        async_nats::jetstream::Context,
    stream:    &'static str,
    subject:   &'static str,
    consumer:  &'static str,
    sale_repo: Arc<dyn sales_service::domain::repositories::SaleRepository>,
    saga_repo: Arc<dyn sales_service::domain::repositories::OrderSagaRepository>,
    outbox:    Arc<dyn ddd_shared_kernel::OutboxRepository>,
    handler:   impl Fn(E, &Arc<dyn sales_service::domain::repositories::SaleRepository>, &Arc<dyn sales_service::domain::repositories::OrderSagaRepository>, &Arc<dyn ddd_shared_kernel::OutboxRepository>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync + 'static,
) where
    E: serde::de::DeserializeOwned + Send + 'static,
{
    tokio::spawn(async move {
        // Get or create stream
        let stream_cfg = async_nats::jetstream::stream::Config {
            name:     stream.to_string(),
            subjects: vec![format!("{}.*.*.*", stream.to_lowercase())],
            ..Default::default()
        };
        let nats_stream = match js.get_or_create_stream(stream_cfg).await {
            Ok(s)  => s,
            Err(e) => { tracing::error!(stream, error = %e, "failed to get/create stream"); return; }
        };
        let consumer_cfg = async_nats::jetstream::consumer::pull::Config {
            durable_name:   Some(consumer.to_string()),
            filter_subject: subject.to_string(),
            ..Default::default()
        };
        let mut msg_stream = match nats_stream.create_consumer(consumer_cfg).await {
            Ok(c)  => match c.messages().await {
                Ok(m)  => m,
                Err(e) => { tracing::error!(consumer, error = %e, "failed to get messages stream"); return; }
            },
            Err(e) => { tracing::error!(consumer, error = %e, "failed to create consumer"); return; }
        };

        tracing::info!(subject, consumer, "NATS subscription active");

        use futures::StreamExt;
        while let Some(msg_result) = msg_stream.next().await {
            match msg_result {
                Ok(msg) => {
                    match serde_json::from_slice::<E>(&msg.payload) {
                        Ok(evt) => {
                            handler(evt, &sale_repo, &saga_repo, &outbox).await;
                            if let Err(e) = msg.ack().await {
                                tracing::warn!(error = %e, "failed to ack NATS message");
                            }
                        }
                        Err(e) => {
                            tracing::warn!(subject, error = %e, "failed to deserialize event, nacking");
                            let _ = msg.ack().await;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(subject, error = %e, "NATS message error");
                }
            }
        }
    });
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
