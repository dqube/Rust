//! Outbox relay wired to a **JetStream** publisher (durable, at-least-once).
//!
//! Mirrors `outbox_relay_setup.rs`, swapping `NatsPublisher` (core NATS,
//! fire-and-forget) for `JetStreamPublisher` (acknowledged publish into a
//! durable JetStream stream). Use this variant when the consumer side
//! needs replay or guaranteed delivery.
//!
//! ```bash
//! DATABASE_URL=postgres://... NATS_URL=nats://localhost:4222 \
//!     cargo run --example outbox_relay_jetstream \
//!     --features "postgres,nats-jetstream"
//! ```

use std::sync::Arc;
use ddd_infrastructure::db::{create_pool, SeaOrmDeadLetterRepository, SeaOrmOutboxRepository};
use ddd_infrastructure::messaging::JetStreamPublisher;
use ddd_shared_kernel::{LogDeadLetterAlert, OutboxRelay};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Connect to Postgres and NATS (with JetStream enabled).
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/postgres".into());
    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".into());

    println!("Connecting to database and NATS/JetStream...");
    let db = create_pool(&db_url).await?;

    // The `service_domain` becomes the JetStream stream prefix
    // (e.g. `ORDERS`) so each service owns its own stream.
    let publisher = Arc::new(JetStreamPublisher::connect(&nats_url, "orders").await?);

    // 2. Repositories (outbox + dead-letter) shared by the relay.
    let outbox_repo = Arc::new(SeaOrmOutboxRepository::new(db.clone()));
    let dead_letter_repo = Arc::new(SeaOrmDeadLetterRepository::new(db.clone()));

    // 3. Wire the relay — same shape as the core-NATS variant.
    let _relay = OutboxRelay::new(
        outbox_repo,
        publisher,
        dead_letter_repo,
        Arc::new(LogDeadLetterAlert),
        10,    // batch size per poll
        1000,  // poll interval (ms)
        5,     // attempts before moving to DLQ
    );

    println!("JetStream-backed outbox relay wired successfully.");

    // 4. Spawn `relay.run()` from your main service task in production.
    Ok(())
}
