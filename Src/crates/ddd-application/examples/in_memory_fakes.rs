//! Demonstrates the in-memory test fakes provided by `ddd-application`.
//!
//! Run with:
//! ```shell
//! cargo run -p ddd-application --example in_memory_fakes --features testing
//! ```

use std::sync::Arc;

use ddd_application::testing::{
    InMemoryDeadLetterRepository, InMemoryIdempotencyStore, InMemoryInboxRepository,
    InMemoryOutboxRepository,
};
use ddd_shared_kernel::{
    dead_letter::{DeadLetterMessage, DeadLetterOrigin, DeadLetterRepository},
    idempotency::IdempotencyStore,
    inbox::{InboxMessage, InboxRepository},
    outbox::{OutboxMessage, OutboxRepository},
};
use serde_json::json;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    // ── Outbox ────────────────────────────────────────────────────────────────
    let outbox = Arc::new(InMemoryOutboxRepository::default());
    let msg_id = Uuid::now_v7();
    outbox
        .save(&OutboxMessage {
            id: msg_id,
            aggregate_id: "order-1".into(),
            aggregate_type: "Order".into(),
            event_type: "order.placed.v1".into(),
            subject: "orders.placed".into(),
            payload: json!({"order_id": "order-1"}),
            created_at: chrono::Utc::now(),
            published_at: None,
            attempts: 0,
            last_error: None,
        })
        .await
        .unwrap();

    println!("Outbox: {} unpublished message(s)", outbox.unpublished().len());
    outbox.mark_as_published(msg_id).await.unwrap();
    println!("Outbox after publish: {} unpublished", outbox.unpublished().len());

    // ── Inbox ─────────────────────────────────────────────────────────────────
    let inbox = Arc::new(InMemoryInboxRepository::default());
    let inbox_id = Uuid::now_v7();
    let first = inbox
        .save(&InboxMessage::new(
            inbox_id,
            "order.placed.v1",
            "orders.placed",
            json!({}),
            "order-service",
        ))
        .await
        .unwrap();
    let dup = inbox
        .save(&InboxMessage::new(
            inbox_id,
            "order.placed.v1",
            "orders.placed",
            json!({}),
            "order-service",
        ))
        .await
        .unwrap();
    println!("\nInbox first insert: {first}  duplicate: {dup}");

    // ── Idempotency ───────────────────────────────────────────────────────────
    let store = Arc::new(InMemoryIdempotencyStore::default());
    let acquired = store
        .try_acquire("req-abc", std::time::Duration::from_secs(60))
        .await
        .unwrap();
    println!("\nIdempotency acquired: {acquired}");
    store.store_response("req-abc", &json!({"id": "order-1"})).await.unwrap();
    let response = store.get_response("req-abc").await.unwrap();
    println!("Idempotency cached response: {:?}", response.map(|r| r.response));

    // ── Dead letter ───────────────────────────────────────────────────────────
    let dlq = Arc::new(InMemoryDeadLetterRepository::default());
    dlq.save(&DeadLetterMessage::new(
        Uuid::now_v7(),
        DeadLetterOrigin::Outbox,
        "order.placed.v1",
        "orders.placed",
        json!({}),
        5,
        "connection refused",
        chrono::Utc::now(),
    ))
    .await
    .unwrap();
    println!("\nDead-letter queue: {} message(s)", dlq.len());
}
