//! Dead Letter Queue (DLQ) support.
//!
//! Messages that exhaust their retry budget in the [`OutboxRelay`] or
//! [`InboxProcessor`] are moved to a dead-letter store and an alert is
//! raised through the [`DeadLetterAlert`] trait.
//!
//! [`OutboxRelay`]: crate::outbox::OutboxRelay
//! [`InboxProcessor`]: crate::inbox::InboxProcessor

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::AppResult;

// ─── DeadLetterMessage ──────────────────────────────────────────────────────

/// Origin of a dead-lettered message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadLetterOrigin {
    /// The message was produced by the outbox (publishing side).
    Outbox,
    /// The message was received through the inbox (consuming side).
    Inbox,
}

impl std::fmt::Display for DeadLetterOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Outbox => f.write_str("outbox"),
            Self::Inbox => f.write_str("inbox"),
        }
    }
}

/// A message that has been moved to the dead-letter store after exhausting
/// all retry attempts.
#[derive(Debug, Clone)]
pub struct DeadLetterMessage {
    /// Unique id for the dead-letter record.
    pub id: Uuid,
    /// The id of the original outbox / inbox message.
    pub original_message_id: Uuid,
    /// Whether this came from the outbox or inbox.
    pub origin: DeadLetterOrigin,
    /// Stable event type string.
    pub event_type: String,
    /// Broker routing key / subject.
    pub subject: String,
    /// Serialised event payload.
    pub payload: Value,
    /// Total number of attempts before dead-lettering.
    pub attempts: u32,
    /// The last error recorded against this message.
    pub last_error: String,
    /// When the original message was created / received.
    pub original_created_at: DateTime<Utc>,
    /// When the message was moved to the dead-letter store.
    pub dead_lettered_at: DateTime<Utc>,
}

impl DeadLetterMessage {
    /// Construct a new dead-letter record.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        original_message_id: Uuid,
        origin: DeadLetterOrigin,
        event_type: impl Into<String>,
        subject: impl Into<String>,
        payload: Value,
        attempts: u32,
        last_error: impl Into<String>,
        original_created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            original_message_id,
            origin,
            event_type: event_type.into(),
            subject: subject.into(),
            payload,
            attempts,
            last_error: last_error.into(),
            original_created_at,
            dead_lettered_at: Utc::now(),
        }
    }
}

// ─── DeadLetterRepository ───────────────────────────────────────────────────

/// Persistence interface for the dead-letter table.
#[async_trait]
pub trait DeadLetterRepository: Send + Sync {
    /// Persist a dead-letter message.
    async fn save(&self, message: &DeadLetterMessage) -> AppResult<()>;

    /// Retrieve dead-letter messages by origin, newest first.
    async fn find_by_origin(
        &self,
        origin: DeadLetterOrigin,
        limit: u32,
    ) -> AppResult<Vec<DeadLetterMessage>>;

    /// Delete dead-letter records older than `older_than`.
    async fn delete_older_than(&self, older_than: DateTime<Utc>) -> AppResult<u64>;
}

// ─── DeadLetterAlert ────────────────────────────────────────────────────────

/// Hook invoked whenever a message is moved to the dead-letter queue.
///
/// Implementations may log a critical-level event, publish a metric, send a
/// notification (PagerDuty, Slack, etc.), or any combination.
#[async_trait]
pub trait DeadLetterAlert: Send + Sync {
    /// Called after a message has been persisted in the dead-letter store.
    async fn on_dead_letter(&self, message: &DeadLetterMessage);
}

/// Default alert implementation that emits a `tracing::error!` event.
pub struct LogDeadLetterAlert;

#[async_trait]
impl DeadLetterAlert for LogDeadLetterAlert {
    async fn on_dead_letter(&self, message: &DeadLetterMessage) {
        tracing::error!(
            dead_letter_id = %message.id,
            original_message_id = %message.original_message_id,
            origin = %message.origin,
            event_type = %message.event_type,
            subject = %message.subject,
            attempts = message.attempts,
            last_error = %message.last_error,
            "Message moved to dead-letter queue"
        );
    }
}
