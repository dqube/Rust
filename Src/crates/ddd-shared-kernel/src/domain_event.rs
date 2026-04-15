//! Domain event building blocks.
//!
//! A *domain event* represents something that happened inside the domain.
//! Events are raised by aggregate roots, collected in memory, and dispatched
//! after the aggregate is persisted.

use std::any::Any;
use std::fmt;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppResult;

// ─── DomainEvent ─────────────────────────────────────────────────────────────

/// Trait that every domain event must implement.
///
/// Implementors should also derive [`Serialize`] / [`Deserialize`] so events
/// can be persisted to the outbox.
pub trait DomainEvent: fmt::Debug + Send + Sync {
    /// A unique, stable name for this event type (e.g. `"user.registered"`).
    fn event_name(&self) -> &'static str;

    /// When the event occurred.
    fn occurred_at(&self) -> DateTime<Utc>;

    /// Downcast support — always implement as `self`.
    fn as_any(&self) -> &dyn Any;
}

// ─── DomainEventEnvelope ─────────────────────────────────────────────────────

/// Wraps a domain event with metadata required for dispatch and persistence.
#[derive(Debug, Serialize, Deserialize)]
pub struct DomainEventEnvelope<E: fmt::Debug> {
    /// Unique envelope / message id.
    pub event_id: Uuid,
    /// The wrapped event payload.
    pub event: E,
    /// String-serialised id of the aggregate that raised the event.
    pub aggregate_id: String,
    /// Type name of the aggregate (e.g. `"Order"`).
    pub aggregate_type: String,
    /// When the event occurred (copied from the event for easy querying).
    pub occurred_at: DateTime<Utc>,
    /// Aggregate version at the time the event was raised.
    pub version: u64,
}

impl<E: DomainEvent> DomainEventEnvelope<E> {
    /// Wrap `event` in an envelope with the given aggregate context.
    pub fn new(
        event: E,
        aggregate_id: impl Into<String>,
        aggregate_type: impl Into<String>,
        version: u64,
    ) -> Self {
        let occurred_at = event.occurred_at();
        Self {
            event_id: Uuid::now_v7(),
            event,
            aggregate_id: aggregate_id.into(),
            aggregate_type: aggregate_type.into(),
            occurred_at,
            version,
        }
    }
}

// ─── DomainEventDispatcher ───────────────────────────────────────────────────

/// Dispatches domain events to registered handlers.
///
/// Implementations are typically provided by the infrastructure layer.
#[async_trait]
pub trait DomainEventDispatcher: Send + Sync {
    /// Dispatch a single event envelope to all registered handlers.
    ///
    /// # Errors
    /// Returns an error when any handler fails.
    async fn dispatch(
        &self,
        envelope: DomainEventEnvelope<Box<dyn DomainEvent>>,
    ) -> AppResult<()>;
}

// ─── DomainEventHandler ──────────────────────────────────────────────────────

/// Handler for a specific domain event type.
///
/// Register implementations with your [`DomainEventDispatcher`].
#[async_trait]
pub trait DomainEventHandler<E: DomainEvent>: Send + Sync {
    /// Handle one event.
    ///
    /// # Errors
    /// Returns an error when handling fails.
    async fn handle(&self, event: &DomainEventEnvelope<E>) -> AppResult<()>;
}
