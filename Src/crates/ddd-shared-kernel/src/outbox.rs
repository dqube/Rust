//! Transactional outbox pattern.
//!
//! The outbox guarantees at-least-once delivery of integration events by
//! persisting them in the same database transaction as the aggregate.  A
//! background [`OutboxRelay`] polls for unpublished messages and forwards them
//! to the configured [`IntegrationEventPublisher`].

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::dead_letter::{DeadLetterAlert, DeadLetterMessage, DeadLetterOrigin, DeadLetterRepository};
use crate::integration_event::IntegrationEventPublisher;
use crate::AppResult;

// ─── OutboxMessage ───────────────────────────────────────────────────────────

/// A message persisted in the outbox table.
#[derive(Debug, Clone)]
pub struct OutboxMessage {
    /// Unique message id (also used as idempotency key).
    pub id: Uuid,
    /// Aggregate id that produced this event.
    pub aggregate_id: String,
    /// Aggregate type name.
    pub aggregate_type: String,
    /// Stable event type string (e.g. `"order.shipped.v1"`).
    pub event_type: String,
    /// Message subject / routing key for the broker.
    pub subject: String,
    /// Serialised event payload.
    pub payload: Value,
    /// When the message was created.
    pub created_at: DateTime<Utc>,
    /// When the message was successfully published (`None` until then).
    pub published_at: Option<DateTime<Utc>>,
    /// How many publish attempts have been made.
    pub attempts: u32,
    /// Last error message, if any.
    pub last_error: Option<String>,
}

impl OutboxMessage {
    /// Construct a new unpublished message.
    pub fn new(
        aggregate_id: impl Into<String>,
        aggregate_type: impl Into<String>,
        event_type: impl Into<String>,
        subject: impl Into<String>,
        payload: Value,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            aggregate_id: aggregate_id.into(),
            aggregate_type: aggregate_type.into(),
            event_type: event_type.into(),
            subject: subject.into(),
            payload,
            created_at: Utc::now(),
            published_at: None,
            attempts: 0,
            last_error: None,
        }
    }

    /// `true` when the message has been successfully published.
    pub fn is_published(&self) -> bool {
        self.published_at.is_some()
    }
}

// ─── OutboxRepository ────────────────────────────────────────────────────────

/// Persistence interface for the outbox table.
#[async_trait]
pub trait OutboxRepository: Send + Sync {
    /// Persist a new message (typically called inside the same transaction as
    /// the aggregate save).
    ///
    /// # Errors
    /// Returns a database error when the insert fails.
    async fn save(&self, message: &OutboxMessage) -> AppResult<()>;

    /// Mark a message as successfully published.
    ///
    /// # Errors
    /// Returns a database error when the update fails.
    async fn mark_as_published(&self, id: Uuid) -> AppResult<()>;

    /// Increment the attempt counter and record an error.
    ///
    /// # Errors
    /// Returns a database error when the update fails.
    async fn mark_as_failed(&self, id: Uuid, error: &str) -> AppResult<()>;

    /// Retrieve up to `limit` messages that have not been published yet.
    ///
    /// # Errors
    /// Returns a database error on query failure.
    async fn find_unpublished(&self, limit: u32) -> AppResult<Vec<OutboxMessage>>;

    /// Delete messages that were published before `older_than`.
    ///
    /// # Errors
    /// Returns a database error when the delete fails.
    async fn delete_published_older_than(&self, older_than: DateTime<Utc>) -> AppResult<u64>;
}

// ─── OutboxRelay ─────────────────────────────────────────────────────────────

/// Background worker that polls the outbox and forwards unpublished messages.
///
/// Typically run as a `tokio::spawn`-ed task alongside the main application.
///
/// Messages that exceed `max_attempts` are moved to a dead-letter store and
/// an alert is raised.
pub struct OutboxRelay {
    repository: Arc<dyn OutboxRepository>,
    publisher: Arc<dyn IntegrationEventPublisher>,
    dead_letter_repo: Arc<dyn DeadLetterRepository>,
    dead_letter_alert: Arc<dyn DeadLetterAlert>,
    /// How many messages to fetch per poll cycle.
    batch_size: u32,
    /// Delay between poll cycles (milliseconds).
    poll_interval_ms: u64,
    /// Move a message to the DLQ after this many failed attempts.
    max_attempts: u32,
}

impl OutboxRelay {
    /// Create a new relay.
    pub fn new(
        repository: Arc<dyn OutboxRepository>,
        publisher: Arc<dyn IntegrationEventPublisher>,
        dead_letter_repo: Arc<dyn DeadLetterRepository>,
        dead_letter_alert: Arc<dyn DeadLetterAlert>,
        batch_size: u32,
        poll_interval_ms: u64,
        max_attempts: u32,
    ) -> Self {
        Self {
            repository,
            publisher,
            dead_letter_repo,
            dead_letter_alert,
            batch_size,
            poll_interval_ms,
            max_attempts,
        }
    }

    /// Run the relay loop indefinitely.
    ///
    /// Call this inside `tokio::spawn`.  The loop stops only when the task is
    /// cancelled.
    pub async fn run(&self) {
        tracing::info!(
            batch_size = self.batch_size,
            poll_interval_ms = self.poll_interval_ms,
            "OutboxRelay started"
        );

        loop {
            if let Err(e) = self.process_batch().await {
                tracing::error!(error = %e, "OutboxRelay batch failed");
            }

            tokio::time::sleep(std::time::Duration::from_millis(self.poll_interval_ms)).await;
        }
    }

    async fn process_batch(&self) -> AppResult<()> {
        let messages = self.repository.find_unpublished(self.batch_size).await?;

        for msg in messages {
            tracing::debug!(
                message_id = %msg.id,
                event_type = %msg.event_type,
                subject = %msg.subject,
                "Publishing outbox message"
            );

            match self.publisher.publish(&msg.subject, &msg.payload).await {
                Ok(()) => {
                    self.repository.mark_as_published(msg.id).await?;
                    tracing::debug!(message_id = %msg.id, "Outbox message published");
                }
                Err(e) => {
                    let err_str = e.to_string();
                    let new_attempts = msg.attempts + 1;
                    tracing::warn!(
                        message_id = %msg.id,
                        attempts = new_attempts,
                        error = %err_str,
                        "Outbox message publish failed"
                    );
                    self.repository.mark_as_failed(msg.id, &err_str).await?;

                    if new_attempts >= self.max_attempts {
                        let dl = DeadLetterMessage::new(
                            msg.id,
                            DeadLetterOrigin::Outbox,
                            &msg.event_type,
                            &msg.subject,
                            msg.payload.clone(),
                            new_attempts,
                            &err_str,
                            msg.created_at,
                        );
                        self.dead_letter_repo.save(&dl).await?;
                        self.dead_letter_alert.on_dead_letter(&dl).await;
                    }
                }
            }
        }

        Ok(())
    }
}
