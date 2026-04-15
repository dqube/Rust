//! NATS subscriber that persists incoming messages into the inbox.

use std::sync::Arc;

use ddd_shared_kernel::{AppError, AppResult, InboxMessage, InboxRepository};
use futures::StreamExt;
use uuid::Uuid;

/// Subscribes to a NATS subject and saves each message into the inbox for
/// idempotent downstream processing.
pub struct NatsSubscriber<I: InboxRepository> {
    client: async_nats::Client,
    inbox_repo: Arc<I>,
    subject: String,
}

impl<I: InboxRepository + 'static> NatsSubscriber<I> {
    /// Build a subscriber.
    pub fn new(client: async_nats::Client, inbox_repo: I, subject: String) -> Self {
        Self { client, inbox_repo: Arc::new(inbox_repo), subject }
    }

    /// Run the subscription loop until the NATS connection closes.
    pub async fn start(self) -> AppResult<()> {
        let mut sub = self
            .client
            .subscribe(self.subject.clone())
            .await
            .map_err(|e| AppError::internal(format!("nats subscribe failed: {e}")))?;

        tracing::info!(subject = %self.subject, "NatsSubscriber started");

        while let Some(msg) = sub.next().await {
            if let Err(e) = self.handle_message(msg).await {
                tracing::warn!(error = %e, "NatsSubscriber: failed to handle message");
            }
        }
        Ok(())
    }

    async fn handle_message(&self, msg: async_nats::Message) -> AppResult<()> {
        let id = extract_id(&msg).unwrap_or_else(Uuid::now_v7);

        let payload: serde_json::Value = serde_json::from_slice(&msg.payload)
            .unwrap_or(serde_json::Value::Null);

        let event_type = header_value(&msg, "event-type").unwrap_or_default();
        let source = header_value(&msg, "source").unwrap_or_default();

        let inbox = InboxMessage::new(id, event_type, msg.subject.as_str(), payload, source);
        let _inserted = self.inbox_repo.save(&inbox).await?;
        Ok(())
    }
}

fn extract_id(msg: &async_nats::Message) -> Option<Uuid> {
    let s = header_value(msg, "message-id")?;
    Uuid::parse_str(&s).ok()
}

fn header_value(msg: &async_nats::Message, key: &str) -> Option<String> {
    msg.headers
        .as_ref()
        .and_then(|h| h.get(key))
        .map(|v| v.to_string())
}
