use std::sync::Arc;
use ddd_infrastructure::db::{SeaOrmOutboxRepository, SeaOrmDeadLetterRepository, create_pool};
use ddd_infrastructure::messaging::NatsPublisher;
use ddd_shared_kernel::{OutboxRelay, LogDeadLetterAlert};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup Database and NATS connections
    // Note: These require actual running instances or will timeout/fail in this example
    let db_url = std::env::var("DATABASE_URL").unwrap_or("postgres://postgres:postgres@localhost:5432/postgres".into());
    let nats_url = std::env::var("NATS_URL").unwrap_or("nats://localhost:4222".into());

    println!("Connecting to database and NATS...");
    let db = create_pool(&db_url).await?;
    let nats = async_nats::connect(nats_url).await?;

    // 2. Initialize repositories and publisher
    let outbox_repo = Arc::new(SeaOrmOutboxRepository::new(db.clone()));
    let dead_letter_repo = Arc::new(SeaOrmDeadLetterRepository::new(db.clone()));
    let publisher = Arc::new(NatsPublisher::new(nats));

    // 3. Create the Relay
    let _relay = OutboxRelay::new(
        outbox_repo,
        publisher,
        dead_letter_repo,
        Arc::new(LogDeadLetterAlert),
        10,    // fetch 10 messages per poll
        1000,  // poll every 1000ms
        5,     // move to DLQ after 5 failed attempts
    );

    println!("Outbox Relay starting...");
    
    // 4. Run the relay loop (usually in a background task)
    // relay.run().await; // This loop is infinite

    // For this example, we just show the wiring.
    println!("Relay wired successfully.");

    Ok(())
}
