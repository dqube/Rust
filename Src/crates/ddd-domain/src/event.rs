//! Domain event routing utilities.
//!
//! [`EventRouter`] lets you register typed handlers for concrete
//! [`DomainEvent`] types and dispatch events by their runtime [`TypeId`].

use std::any::{Any, TypeId};
use std::collections::HashMap;

use async_trait::async_trait;
use ddd_shared_kernel::{AppResult, DomainEvent, DomainEventDispatcher, DomainEventEnvelope};

/// Convenience extensions on [`DomainEventDispatcher`].
#[async_trait]
pub trait DomainEventDispatcherExt: DomainEventDispatcher {
    /// Dispatch many envelopes sequentially, short-circuiting on the first
    /// handler error.
    async fn dispatch_all(
        &self,
        envelopes: Vec<DomainEventEnvelope<Box<dyn DomainEvent>>>,
    ) -> AppResult<()> {
        for env in envelopes {
            self.dispatch(env).await?;
        }
        Ok(())
    }
}

impl<T: DomainEventDispatcher + ?Sized> DomainEventDispatcherExt for T {}

type HandlerFn = Box<dyn Fn(&dyn Any) + Send + Sync>;

/// A synchronous in-process router that dispatches a domain event to one or
/// more handlers keyed by its concrete [`TypeId`].
///
/// Handlers receive the event as `&dyn Any` and are expected to downcast.
#[derive(Default)]
pub struct EventRouter {
    handlers: HashMap<TypeId, Vec<HandlerFn>>,
}

impl EventRouter {
    /// Create an empty router.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler for a specific event type.
    pub fn on<E, H>(&mut self, handler: H)
    where
        E: DomainEvent + 'static,
        H: Fn(&E) + Send + Sync + 'static,
    {
        let entry = self.handlers.entry(TypeId::of::<E>()).or_default();
        entry.push(Box::new(move |any| {
            if let Some(ev) = any.downcast_ref::<E>() {
                handler(ev);
            }
        }));
    }

    /// Dispatch `event` to all registered handlers for its concrete type.
    ///
    /// Returns the number of handlers invoked.
    pub fn dispatch<E: DomainEvent + 'static>(&self, event: &E) -> usize {
        let Some(hs) = self.handlers.get(&TypeId::of::<E>()) else {
            return 0;
        };
        for h in hs {
            h(event as &dyn Any);
        }
        hs.len()
    }

    /// Dispatch a boxed-dyn event. Uses [`DomainEvent::as_any`] to recover the
    /// concrete [`TypeId`].
    pub fn dispatch_dyn(&self, event: &dyn DomainEvent) -> usize {
        let any = event.as_any();
        let Some(hs) = self.handlers.get(&any.type_id()) else {
            return 0;
        };
        for h in hs {
            h(any);
        }
        hs.len()
    }
}

/// Stub publisher that persists integration events to an outbox.
///
/// Concrete implementations typically wrap
/// [`ddd_shared_kernel::OutboxRepository`] and serialise the event payload.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish a single domain event envelope.
    async fn publish(
        &self,
        envelope: DomainEventEnvelope<Box<dyn DomainEvent>>,
    ) -> AppResult<()>;
}

/// Register a handler on an [`EventRouter`] for a given concrete event type.
///
/// # Example
/// ```
/// use ddd_domain::{register_event_handler, event::EventRouter};
/// use ddd_shared_kernel::DomainEvent;
/// use chrono::{DateTime, Utc};
/// use std::any::Any;
///
/// #[derive(Debug)]
/// struct OrderCreated { at: DateTime<Utc> }
/// impl DomainEvent for OrderCreated {
///     fn event_name(&self) -> &'static str { "order.created" }
///     fn occurred_at(&self) -> DateTime<Utc> { self.at }
///     fn as_any(&self) -> &dyn Any { self }
/// }
///
/// let mut router = EventRouter::new();
/// register_event_handler!(router, OrderCreated, |e| {
///     let _ = e.event_name();
/// });
/// ```
#[macro_export]
macro_rules! register_event_handler {
    ($router:expr, $event:ty, $handler:expr) => {
        $router.on::<$event, _>($handler)
    };
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use chrono::{DateTime, Utc};
    use ddd_shared_kernel::DomainEvent;

    use super::*;

    #[derive(Debug)]
    struct Created {
        at: DateTime<Utc>,
    }
    impl DomainEvent for Created {
        fn event_name(&self) -> &'static str {
            "test.created"
        }
        fn occurred_at(&self) -> DateTime<Utc> {
            self.at
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn router_dispatches_typed() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut router = EventRouter::new();
        let c = counter.clone();
        router.on::<Created, _>(move |_e| {
            c.fetch_add(1, Ordering::SeqCst);
        });
        let n = router.dispatch(&Created { at: Utc::now() });
        assert_eq!(n, 1);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn router_dispatches_dyn() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut router = EventRouter::new();
        let c = counter.clone();
        router.on::<Created, _>(move |_e| {
            c.fetch_add(1, Ordering::SeqCst);
        });
        let ev: Box<dyn DomainEvent> = Box::new(Created { at: Utc::now() });
        let n = router.dispatch_dyn(ev.as_ref());
        assert_eq!(n, 1);
    }
}
