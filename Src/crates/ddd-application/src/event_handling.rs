//! Domain-event handler registry for the application layer.
//!
//! [`EventHandlerRegistry`] is keyed on the concrete [`TypeId`] of a
//! [`DomainEvent`] and dispatches a boxed-dyn event to every handler
//! registered for that type.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use ddd_shared_kernel::{AppError, AppResult, DomainEvent, DomainEventEnvelope};

/// Convenience alias for a boxed domain event.
pub type BoxedDomainEvent = Box<dyn DomainEvent>;

/// Handler for a specific domain-event type.
#[async_trait]
pub trait DomainEventHandler<E: DomainEvent>: Send + Sync {
    /// Handle the event envelope.
    async fn handle(&self, event: DomainEventEnvelope<E>) -> AppResult<()>;
}

// ── Internal erased dispatcher ───────────────────────────────────────────────

#[async_trait]
trait ErasedHandler: Send + Sync {
    async fn dispatch(
        &self,
        event: &dyn DomainEvent,
        aggregate_id: String,
        aggregate_type: String,
        version: u64,
    ) -> AppResult<()>;
}

struct Typed<E: DomainEvent, H: DomainEventHandler<E>> {
    inner: Arc<H>,
    _e: std::marker::PhantomData<fn() -> E>,
}

#[async_trait]
impl<E, H> ErasedHandler for Typed<E, H>
where
    E: DomainEvent + Clone + 'static,
    H: DomainEventHandler<E> + 'static,
{
    async fn dispatch(
        &self,
        event: &dyn DomainEvent,
        aggregate_id: String,
        aggregate_type: String,
        version: u64,
    ) -> AppResult<()> {
        let e = event
            .as_any()
            .downcast_ref::<E>()
            .ok_or_else(|| AppError::internal("domain event type mismatch"))?
            .clone();
        let envelope = DomainEventEnvelope::new(e, aggregate_id, aggregate_type, version);
        self.inner.handle(envelope).await
    }
}

// ── Registry ────────────────────────────────────────────────────────────────

/// Registry of typed domain-event handlers. Multiple handlers may be
/// registered for the same event type; they are invoked in registration order
/// and the first error short-circuits further dispatch.
#[derive(Default)]
pub struct EventHandlerRegistry {
    handlers: HashMap<TypeId, Vec<Arc<dyn ErasedHandler>>>,
}

impl EventHandlerRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler for event type `E`.
    pub fn register<E, H>(&mut self, handler: H)
    where
        E: DomainEvent + Clone + 'static,
        H: DomainEventHandler<E> + 'static,
    {
        let typed: Arc<dyn ErasedHandler> = Arc::new(Typed::<E, H> {
            inner: Arc::new(handler),
            _e: std::marker::PhantomData,
        });
        self.handlers.entry(TypeId::of::<E>()).or_default().push(typed);
    }

    /// Dispatch a boxed event to every handler registered for its concrete
    /// [`TypeId`]. Unknown event types are silently skipped.
    pub async fn dispatch(
        &self,
        event: BoxedDomainEvent,
        aggregate_id: String,
        aggregate_type: String,
    ) -> AppResult<()> {
        self.dispatch_versioned(event, aggregate_id, aggregate_type, 0).await
    }

    /// Same as [`dispatch`](Self::dispatch) but lets the caller specify the
    /// aggregate version recorded in the envelope.
    pub async fn dispatch_versioned(
        &self,
        event: BoxedDomainEvent,
        aggregate_id: String,
        aggregate_type: String,
        version: u64,
    ) -> AppResult<()> {
        let tid = event.as_any().type_id();
        let Some(hs) = self.handlers.get(&tid) else {
            return Ok(());
        };
        for h in hs {
            h.dispatch(event.as_ref(), aggregate_id.clone(), aggregate_type.clone(), version)
                .await?;
        }
        Ok(())
    }
}

/// Register an event handler with [`EventHandlerRegistry`].
///
/// # Example
/// ```ignore
/// use ddd_application::register_handler;
/// register_handler!(registry, OrderPlaced => OrderPlacedHandler);
/// ```
#[macro_export]
macro_rules! register_handler {
    ($registry:expr, $event:ty => $handler:expr) => {
        $registry.register::<$event, _>($handler)
    };
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use chrono::{DateTime, Utc};

    use super::*;

    #[derive(Debug, Clone)]
    struct Created {
        at: DateTime<Utc>,
    }
    impl DomainEvent for Created {
        fn event_name(&self) -> &'static str {
            "x.created"
        }
        fn occurred_at(&self) -> DateTime<Utc> {
            self.at
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    struct H(Arc<AtomicUsize>);

    #[async_trait]
    impl DomainEventHandler<Created> for H {
        async fn handle(&self, _e: DomainEventEnvelope<Created>) -> AppResult<()> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn dispatches_to_registered_handler() {
        let count = Arc::new(AtomicUsize::new(0));
        let mut reg = EventHandlerRegistry::new();
        reg.register::<Created, _>(H(count.clone()));

        let ev: BoxedDomainEvent = Box::new(Created { at: Utc::now() });
        reg.dispatch(ev, "agg-1".into(), "Order".into()).await.unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn unknown_event_type_is_noop() {
        let reg = EventHandlerRegistry::new();
        let ev: BoxedDomainEvent = Box::new(Created { at: Utc::now() });
        reg.dispatch(ev, "id".into(), "T".into()).await.unwrap();
    }
}
