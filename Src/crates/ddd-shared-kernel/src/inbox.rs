//! Idempotent inbox pattern.
//!
//! The inbox guarantees exactly-once processing of incoming integration events.
//! Each received message is stored before processing so that duplicate
//! deliveries from the broker are detected and ignored.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::dead_letter::{DeadLetterAlert, DeadLetterMessage, DeadLetterOrigin, DeadLetterRepository};
use crate::AppResult;

// ─── InboxMessage ────────────────────────────────────────────────────────────

/// A message received from the broker and persisted in the inbox table.
#[derive(Debug, Clone)]
pub struct InboxMessage {
    /// Unique message id (the broker-assigned idempotency key).
    pub id: Uuid,
    /// Stable event type string.
    pub event_type: String,
    /// Message subject / routing key.
    pub subject: String,
    /// Serialised event payload.
    pub payload: Value,
    /// Source service that published the event.
    pub source: String,
    /// When the message was received.
    pub received_at: DateTime<Utc>,
    /// When the message was successfully processed (`None` until then).
    pub processed_at: Option<DateTime<Utc>>,
    /// How many processing attempts have been made.
    pub attempts: u32,
    /// Last error message, if any.
    pub last_error: Option<String>,
}

impl InboxMessage {
    /// Construct a new unprocessed message.
    pub fn new(
        id: Uuid,
        event_type: impl Into<String>,
        subject: impl Into<String>,
        payload: Value,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id,
            event_type: event_type.into(),
            subject: subject.into(),
            payload,
            source: source.into(),
            received_at: Utc::now(),
            processed_at: None,
            attempts: 0,
            last_error: None,
        }
    }

    /// `true` when the message has been successfully processed.
    pub fn is_processed(&self) -> bool {
        self.processed_at.is_some()
    }
}

// ─── InboxRepository ─────────────────────────────────────────────────────────

/// Persistence interface for the inbox table.
#[async_trait]
pub trait InboxRepository: Send + Sync {
    /// Persist an incoming message.  Returns `false` when the message already
    /// exists (duplicate delivery).
    ///
    /// # Errors
    /// Returns a database error when the insert fails.
    async fn save(&self, message: &InboxMessage) -> AppResult<bool>;

    /// Mark a message as successfully processed.
    ///
    /// # Errors
    /// Returns a database error when the update fails.
    async fn mark_as_processed(&self, id: Uuid) -> AppResult<()>;

    /// Increment the attempt counter and record an error.
    ///
    /// # Errors
    /// Returns a database error when the update fails.
    async fn mark_as_failed(&self, id: Uuid, error: &str) -> AppResult<()>;

    /// Retrieve up to `limit` messages that have not yet been processed.
    ///
    /// # Errors
    /// Returns a database error on query failure.
    async fn find_unprocessed(&self, limit: u32) -> AppResult<Vec<InboxMessage>>;

    /// Delete messages processed before `older_than`.
    ///
    /// # Errors
    /// Returns a database error when the delete fails.
    async fn delete_processed_older_than(&self, older_than: DateTime<Utc>) -> AppResult<u64>;
}

// ─── InboxMessageHandler ─────────────────────────────────────────────────────

/// Processes a single inbox message.
///
/// Implement this trait for each event type your service handles.
#[async_trait]
pub trait InboxMessageHandler: Send + Sync {
    /// The event type string this handler cares about (e.g. `"order.shipped.v1"`).
    fn handles_event_type(&self) -> &'static str;

    /// Process the message.
    ///
    /// # Errors
    /// Returns an error when processing fails; the inbox processor will record
    /// the failure and retry.
    async fn handle(&self, message: &InboxMessage) -> AppResult<()>;
}

// ─── InboxProcessor ──────────────────────────────────────────────────────────

/// Background worker that polls the inbox and dispatches unprocessed messages
/// to the appropriate [`InboxMessageHandler`].
///
/// Messages that exceed `max_attempts` are moved to a dead-letter store and
/// an alert is raised.
pub struct InboxProcessor {
    repository: Arc<dyn InboxRepository>,
    handlers: Vec<Arc<dyn InboxMessageHandler>>,
    dead_letter_repo: Arc<dyn DeadLetterRepository>,
    dead_letter_alert: Arc<dyn DeadLetterAlert>,
    /// How many messages to fetch per poll cycle.
    batch_size: u32,
    /// Delay between poll cycles (milliseconds).
    poll_interval_ms: u64,
    /// Move a message to the DLQ after this many failed attempts.
    max_attempts: u32,
}

impl InboxProcessor {
    /// Create a new processor.
    pub fn new(
        repository: Arc<dyn InboxRepository>,
        handlers: Vec<Arc<dyn InboxMessageHandler>>,
        dead_letter_repo: Arc<dyn DeadLetterRepository>,
        dead_letter_alert: Arc<dyn DeadLetterAlert>,
        batch_size: u32,
        poll_interval_ms: u64,
        max_attempts: u32,
    ) -> Self {
        Self {
            repository,
            handlers,
            dead_letter_repo,
            dead_letter_alert,
            batch_size,
            poll_interval_ms,
            max_attempts,
        }
    }

    /// Run the processor loop indefinitely.
    ///
    /// Call this inside `tokio::spawn`.
    pub async fn run(&self) {
        tracing::info!(
            batch_size = self.batch_size,
            poll_interval_ms = self.poll_interval_ms,
            "InboxProcessor started"
        );

        loop {
            if let Err(e) = self.process_batch().await {
                tracing::error!(error = %e, "InboxProcessor batch failed");
            }

            tokio::time::sleep(std::time::Duration::from_millis(self.poll_interval_ms)).await;
        }
    }

    async fn process_batch(&self) -> AppResult<()> {
        let messages = self.repository.find_unprocessed(self.batch_size).await?;

        for msg in messages {
            let handler = self
                .handlers
                .iter()
                .find(|h| h.handles_event_type() == msg.event_type.as_str());

            match handler {
                None => {
                    tracing::warn!(
                        message_id = %msg.id,
                        event_type = %msg.event_type,
                        "No handler registered for event type — skipping"
                    );
                }
                Some(h) => {
                    tracing::debug!(
                        message_id = %msg.id,
                        event_type = %msg.event_type,
                        "Processing inbox message"
                    );

                    match h.handle(&msg).await {
                        Ok(()) => {
                            self.repository.mark_as_processed(msg.id).await?;
                            tracing::debug!(message_id = %msg.id, "Inbox message processed");
                        }
                        Err(e) => {
                            let err_str = e.to_string();
                            let new_attempts = msg.attempts + 1;
                            tracing::warn!(
                                message_id = %msg.id,
                                attempts = new_attempts,
                                error = %err_str,
                                "Inbox message processing failed"
                            );
                            self.repository.mark_as_failed(msg.id, &err_str).await?;

                            if new_attempts >= self.max_attempts {
                                let dl = DeadLetterMessage::new(
                                    msg.id,
                                    DeadLetterOrigin::Inbox,
                                    &msg.event_type,
                                    &msg.subject,
                                    msg.payload.clone(),
                                    new_attempts,
                                    &err_str,
                                    msg.received_at,
                                );
                                self.dead_letter_repo.save(&dl).await?;
                                self.dead_letter_alert.on_dead_letter(&dl).await;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
