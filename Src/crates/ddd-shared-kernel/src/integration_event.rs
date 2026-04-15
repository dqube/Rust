//! Integration event building blocks.
//!
//! An *integration event* crosses service boundaries.  Unlike domain events,
//! integration events are serialised and published to a message broker (e.g.
//! NATS, Kafka, RabbitMQ).

use std::fmt;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::AppResult;

// ─── IntegrationEvent ────────────────────────────────────────────────────────

/// Trait that every integration event must implement.
pub trait IntegrationEvent: fmt::Debug + Send + Sync + Serialize {
    /// Stable event-type string (e.g. `"order.shipped.v1"`).
    fn event_type(&self) -> &'static str;

    /// The message subject / routing key (e.g. `"orders.123.shipped"`).
    fn subject(&self) -> String;

    /// When the event occurred.
    fn occurred_at(&self) -> DateTime<Utc>;
}

// ─── IntegrationEventEnvelope ────────────────────────────────────────────────

/// Wraps an integration event with transport-level metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationEventEnvelope {
    /// Unique envelope id for idempotent processing.
    pub event_id: Uuid,
    /// Stable event type string.
    pub event_type: String,
    /// Message subject / routing key.
    pub subject: String,
    /// Serialised event payload.
    pub payload: Value,
    /// Source service name.
    pub source: String,
    /// When the event occurred.
    pub occurred_at: DateTime<Utc>,
    /// Schema version for forward compatibility.
    pub schema_version: u32,
}

impl IntegrationEventEnvelope {
    /// Wrap an [`IntegrationEvent`] implementation.
    ///
    /// # Errors
    /// Returns an error when the event cannot be serialised to JSON.
    pub fn new<E: IntegrationEvent>(
        event: &E,
        source: impl Into<String>,
        schema_version: u32,
    ) -> Result<Self, serde_json::Error> {
        Ok(Self {
            event_id: Uuid::now_v7(),
            event_type: event.event_type().to_owned(),
            subject: event.subject(),
            payload: serde_json::to_value(event)?,
            source: source.into(),
            occurred_at: event.occurred_at(),
            schema_version,
        })
    }
}

// ─── IntegrationEventPublisher ───────────────────────────────────────────────

/// Publishes integration events to a message broker.
///
/// Infrastructure implementations wrap NATS, Kafka, etc.
#[async_trait]
pub trait IntegrationEventPublisher: Send + Sync {
    /// Publish a raw JSON payload to `subject`.
    ///
    /// # Errors
    /// Returns an error when the broker is unavailable or rejects the message.
    async fn publish(&self, subject: &str, payload: &Value) -> AppResult<()>;

    /// Publish an [`IntegrationEventEnvelope`].
    ///
    /// The default implementation serialises the envelope and calls
    /// [`Self::publish`].
    ///
    /// # Errors
    /// Returns an error when serialisation or publishing fails.
    async fn publish_envelope(&self, envelope: &IntegrationEventEnvelope) -> AppResult<()> {
        let payload = serde_json::to_value(envelope)
            .map_err(|e| crate::AppError::serialization(e.to_string()))?;
        self.publish(&envelope.subject, &payload).await
    }
}
