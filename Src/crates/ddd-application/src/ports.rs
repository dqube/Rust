//! Infrastructure ports — clock, id generator, integration-event publisher.
//!
//! Application code should depend on these traits, not on concrete
//! implementations, so tests can inject deterministic stand-ins.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ddd_shared_kernel::{AppResult, IntegrationEvent};
use uuid::Uuid;

// ─── Clock ───────────────────────────────────────────────────────────────────

/// Abstraction over "now".
pub trait Clock: Send + Sync {
    /// Current UTC timestamp.
    fn now(&self) -> DateTime<Utc>;
}

/// System wall-clock implementation of [`Clock`].
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

// ─── IdGenerator ─────────────────────────────────────────────────────────────

/// Abstraction over id generation.
pub trait IdGenerator: Send + Sync {
    /// Produce a fresh identifier.
    fn generate(&self) -> Uuid;
}

/// Generates UUID v7 (time-ordered) ids.
#[derive(Debug, Default, Clone, Copy)]
pub struct UuidV7Generator;

impl IdGenerator for UuidV7Generator {
    fn generate(&self) -> Uuid {
        Uuid::now_v7()
    }
}

// ─── EventPublisher ──────────────────────────────────────────────────────────

/// Publishes integration events to the outside world (broker / outbox).
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish one integration event.
    async fn publish<E: IntegrationEvent>(&self, event: E) -> AppResult<()>;
}

/// No-op publisher for tests and local development.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullEventPublisher;

#[async_trait]
impl EventPublisher for NullEventPublisher {
    async fn publish<E: IntegrationEvent>(&self, _event: E) -> AppResult<()> {
        Ok(())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_returns_now() {
        let before = Utc::now();
        let t = SystemClock.now();
        let after = Utc::now();
        assert!(t >= before && t <= after);
    }

    #[test]
    fn uuid_v7_generator_emits_unique_ids() {
        let g = UuidV7Generator;
        assert_ne!(g.generate(), g.generate());
    }
}
