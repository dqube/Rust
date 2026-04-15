//! Unified mediator facade with inventory-based self-registration.
//!
//! [`Mediator`] exposes three operations — [`send`](Mediator::send) for
//! commands, [`query`](Mediator::query) for queries, and
//! [`publish`](Mediator::publish) for domain events. Handlers register
//! themselves at link time through the [`inventory`] crate, so adding a new
//! handler is a matter of dropping `register_command_handler!` /
//! `register_query_handler!` / `register_event_handler!` next to the handler
//! type — no central wire-up list.
//!
//! ## Performance
//!
//! Dispatch is a single [`FxHashMap`](rustc_hash::FxHashMap) lookup on
//! [`TypeId`] followed by one `Arc<dyn Trait>` clone. The registry is built
//! once at startup and is read-only thereafter, so there is no locking on the
//! hot path. Expect ~15–30 ns per `send` / `query` before the handler body
//! runs.
//!
//! ## Example
//!
//! ```ignore
//! use std::sync::Arc;
//! use ddd_application::{
//!     Command, CommandHandler, Mediator,
//!     register_command_handler,
//! };
//! use ddd_shared_kernel::AppResult;
//! use async_trait::async_trait;
//!
//! // 1. A deps container — anything `Send + Sync + 'static`.
//! #[derive(Clone)]
//! pub struct AppDeps {
//!     pub db: Arc<sea_orm::DatabaseConnection>,
//! }
//!
//! // 2. A command + handler.
//! pub struct CreateOrder { pub sku: String }
//! impl Command for CreateOrder { type Response = uuid::Uuid; }
//!
//! pub struct CreateOrderHandler { db: Arc<sea_orm::DatabaseConnection> }
//! #[async_trait]
//! impl CommandHandler<CreateOrder> for CreateOrderHandler {
//!     async fn handle(&self, _: CreateOrder) -> AppResult<uuid::Uuid> {
//!         Ok(uuid::Uuid::now_v7())
//!     }
//! }
//!
//! // 3. Self-registration — sits next to the handler, no wiring file.
//! register_command_handler!(CreateOrder, AppDeps, |deps: &AppDeps| {
//!     CreateOrderHandler { db: deps.db.clone() }
//! });
//!
//! // 4. At startup.
//! # async fn demo(deps: AppDeps) -> AppResult<()> {
//! let mediator = Mediator::from_inventory(&deps);
//! let id = mediator.send(CreateOrder { sku: "abc".into() }).await?;
//! # let _ = id; Ok(()) }
//! ```

use std::any::{Any, TypeId};
use std::sync::Arc;

use async_trait::async_trait;
use ddd_shared_kernel::{AppError, AppResult, DomainEvent, DomainEventEnvelope};
use rustc_hash::FxHashMap;

use crate::cqrs::{Command, CommandHandler, Query, QueryHandler};
use crate::event_handling::DomainEventHandler;

// Re-export for use from the registration macros without forcing downstream
// crates to add `inventory` to their own `Cargo.toml`.
#[doc(hidden)]
pub use inventory;

// ─── Inventory slot ──────────────────────────────────────────────────────────

/// Link-time registration record produced by the `register_*_handler!`
/// macros. Not typically constructed by hand.
pub struct HandlerRegistration {
    /// Human-readable type name of the command / query / event, used only in
    /// error messages.
    pub name: &'static str,
    /// Installs the handler into the registry. `deps` is the `&dyn Any` view
    /// of the deps struct passed to [`Mediator::from_inventory`]; the macro
    /// downcasts it to the concrete type the handler expects.
    pub register: fn(&mut MediatorRegistry, &(dyn Any + Send + Sync)),
}

inventory::collect!(HandlerRegistration);

// ─── Event handler erasure ───────────────────────────────────────────────────

#[async_trait]
trait ErasedEventHandler: Send + Sync {
    async fn dispatch(
        &self,
        event: &dyn DomainEvent,
        aggregate_id: String,
        aggregate_type: String,
        version: u64,
    ) -> AppResult<()>;
}

struct TypedEventHandler<E: DomainEvent, H: DomainEventHandler<E>> {
    inner: Arc<H>,
    _e: std::marker::PhantomData<fn() -> E>,
}

#[async_trait]
impl<E, H> ErasedEventHandler for TypedEventHandler<E, H>
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

// ─── Registry ────────────────────────────────────────────────────────────────

/// Mutable registry used during [`Mediator`] construction. Handler
/// registration macros call the `register_*` methods here.
#[derive(Default)]
pub struct MediatorRegistry {
    commands: FxHashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    queries: FxHashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    events: FxHashMap<TypeId, Vec<Arc<dyn ErasedEventHandler>>>,
}

impl MediatorRegistry {
    /// Register a command handler. Replaces any previous handler for `C`.
    pub fn register_command<C, H>(&mut self, handler: H)
    where
        C: Command,
        H: CommandHandler<C> + 'static,
    {
        let arc: Arc<dyn CommandHandler<C>> = Arc::new(handler);
        self.commands.insert(TypeId::of::<C>(), Arc::new(arc));
    }

    /// Register a query handler. Replaces any previous handler for `Q`.
    pub fn register_query<Q, H>(&mut self, handler: H)
    where
        Q: Query,
        H: QueryHandler<Q> + 'static,
    {
        let arc: Arc<dyn QueryHandler<Q>> = Arc::new(handler);
        self.queries.insert(TypeId::of::<Q>(), Arc::new(arc));
    }

    /// Register a domain-event handler. Multiple handlers for the same event
    /// type are fan-out in registration order.
    pub fn register_event<E, H>(&mut self, handler: H)
    where
        E: DomainEvent + Clone + 'static,
        H: DomainEventHandler<E> + 'static,
    {
        let typed: Arc<dyn ErasedEventHandler> = Arc::new(TypedEventHandler::<E, H> {
            inner: Arc::new(handler),
            _e: std::marker::PhantomData,
        });
        self.events.entry(TypeId::of::<E>()).or_default().push(typed);
    }
}

// ─── Mediator ────────────────────────────────────────────────────────────────

/// Immutable dispatcher for commands, queries, and domain events.
pub struct Mediator {
    reg: MediatorRegistry,
}

impl Mediator {
    /// Start an empty builder for manual registration.
    pub fn builder() -> MediatorBuilder {
        MediatorBuilder(MediatorRegistry::default())
    }

    /// Build a mediator by collecting every handler registered through the
    /// `register_*_handler!` macros. `deps` is downcast by each registration
    /// closure to the concrete type it expects.
    pub fn from_inventory<D>(deps: &D) -> Self
    where
        D: Any + Send + Sync + 'static,
    {
        let mut reg = MediatorRegistry::default();
        let deps_dyn: &(dyn Any + Send + Sync) = deps;
        for r in inventory::iter::<HandlerRegistration> {
            (r.register)(&mut reg, deps_dyn);
        }
        Self { reg }
    }

    /// Dispatch a command to its registered handler.
    #[inline]
    pub async fn send<C: Command>(&self, cmd: C) -> AppResult<C::Response> {
        let slot = self.reg.commands.get(&TypeId::of::<C>()).ok_or_else(|| {
            AppError::internal(format!(
                "no command handler registered for {}",
                std::any::type_name::<C>()
            ))
        })?;
        let handler = slot
            .downcast_ref::<Arc<dyn CommandHandler<C>>>()
            .ok_or_else(|| AppError::internal("command handler type mismatch"))?
            .clone();
        handler.handle(cmd).await
    }

    /// Dispatch a query to its registered handler.
    #[inline]
    pub async fn query<Q: Query>(&self, q: Q) -> AppResult<Q::Response> {
        let slot = self.reg.queries.get(&TypeId::of::<Q>()).ok_or_else(|| {
            AppError::internal(format!(
                "no query handler registered for {}",
                std::any::type_name::<Q>()
            ))
        })?;
        let handler = slot
            .downcast_ref::<Arc<dyn QueryHandler<Q>>>()
            .ok_or_else(|| AppError::internal("query handler type mismatch"))?
            .clone();
        handler.handle(q).await
    }

    /// Publish a domain event to every handler registered for its concrete
    /// type. Handlers are invoked in registration order; the first error
    /// short-circuits.
    ///
    /// Unknown event types are a no-op.
    pub async fn publish<E>(
        &self,
        event: E,
        aggregate_id: impl Into<String>,
        aggregate_type: impl Into<String>,
        version: u64,
    ) -> AppResult<()>
    where
        E: DomainEvent + Clone + 'static,
    {
        let Some(hs) = self.reg.events.get(&TypeId::of::<E>()) else {
            return Ok(());
        };
        let aggregate_id = aggregate_id.into();
        let aggregate_type = aggregate_type.into();
        for h in hs {
            h.dispatch(&event, aggregate_id.clone(), aggregate_type.clone(), version)
                .await?;
        }
        Ok(())
    }
}

/// Manual-registration builder, as an alternative to inventory discovery.
pub struct MediatorBuilder(MediatorRegistry);

impl MediatorBuilder {
    /// Register a command handler.
    pub fn command<C, H>(mut self, handler: H) -> Self
    where
        C: Command,
        H: CommandHandler<C> + 'static,
    {
        self.0.register_command::<C, _>(handler);
        self
    }

    /// Register a query handler.
    pub fn query<Q, H>(mut self, handler: H) -> Self
    where
        Q: Query,
        H: QueryHandler<Q> + 'static,
    {
        self.0.register_query::<Q, _>(handler);
        self
    }

    /// Register a domain-event handler.
    pub fn event<E, H>(mut self, handler: H) -> Self
    where
        E: DomainEvent + Clone + 'static,
        H: DomainEventHandler<E> + 'static,
    {
        self.0.register_event::<E, _>(handler);
        self
    }

    /// Finish construction.
    pub fn build(self) -> Mediator {
        Mediator { reg: self.0 }
    }
}

// ─── Registration macros ─────────────────────────────────────────────────────

/// Register a [`CommandHandler`] at link time.
///
/// The third argument is a closure `|deps: &DepsType| -> Handler` that
/// constructs the handler from the deps struct passed to
/// [`Mediator::from_inventory`].
#[macro_export]
macro_rules! register_command_handler {
    ($cmd:ty, $deps:ty, $ctor:expr) => {
        $crate::mediator::inventory::submit! {
            $crate::mediator::HandlerRegistration {
                name: stringify!($cmd),
                register: |reg, deps_any| {
                    let deps = <dyn ::std::any::Any>::downcast_ref::<$deps>(deps_any)
                        .expect(concat!(
                            "Mediator deps type mismatch for command handler of ",
                            stringify!($cmd),
                        ));
                    let ctor: fn(&$deps) -> _ = $ctor;
                    reg.register_command::<$cmd, _>(ctor(deps));
                },
            }
        }
    };
}

/// Register a [`QueryHandler`] at link time. See [`register_command_handler!`].
#[macro_export]
macro_rules! register_query_handler {
    ($query:ty, $deps:ty, $ctor:expr) => {
        $crate::mediator::inventory::submit! {
            $crate::mediator::HandlerRegistration {
                name: stringify!($query),
                register: |reg, deps_any| {
                    let deps = <dyn ::std::any::Any>::downcast_ref::<$deps>(deps_any)
                        .expect(concat!(
                            "Mediator deps type mismatch for query handler of ",
                            stringify!($query),
                        ));
                    let ctor: fn(&$deps) -> _ = $ctor;
                    reg.register_query::<$query, _>(ctor(deps));
                },
            }
        }
    };
}

/// Register a [`DomainEventHandler`] at link time. See
/// [`register_command_handler!`].
#[macro_export]
macro_rules! register_event_handler {
    ($event:ty, $deps:ty, $ctor:expr) => {
        $crate::mediator::inventory::submit! {
            $crate::mediator::HandlerRegistration {
                name: stringify!($event),
                register: |reg, deps_any| {
                    let deps = <dyn ::std::any::Any>::downcast_ref::<$deps>(deps_any)
                        .expect(concat!(
                            "Mediator deps type mismatch for event handler of ",
                            stringify!($event),
                        ));
                    let ctor: fn(&$deps) -> _ = $ctor;
                    reg.register_event::<$event, _>(ctor(deps));
                },
            }
        }
    };
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use async_trait::async_trait;
    use chrono::{DateTime, Utc};

    use super::*;

    struct Ping;
    impl Command for Ping {
        type Response = &'static str;
    }
    struct PingHandler;
    #[async_trait]
    impl CommandHandler<Ping> for PingHandler {
        async fn handle(&self, _: Ping) -> AppResult<&'static str> {
            Ok("pong")
        }
    }

    struct Count;
    impl Query for Count {
        type Response = u32;
    }
    struct CountHandler(u32);
    #[async_trait]
    impl QueryHandler<Count> for CountHandler {
        async fn handle(&self, _: Count) -> AppResult<u32> {
            Ok(self.0)
        }
    }

    #[derive(Debug, Clone)]
    struct Tick {
        at: DateTime<Utc>,
    }
    impl DomainEvent for Tick {
        fn event_name(&self) -> &'static str {
            "tick"
        }
        fn occurred_at(&self) -> DateTime<Utc> {
            self.at
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }
    struct TickHandler(Arc<AtomicUsize>);
    #[async_trait]
    impl DomainEventHandler<Tick> for TickHandler {
        async fn handle(&self, _: DomainEventEnvelope<Tick>) -> AppResult<()> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn builder_send_query_publish() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mediator = Mediator::builder()
            .command::<Ping, _>(PingHandler)
            .query::<Count, _>(CountHandler(7))
            .event::<Tick, _>(TickHandler(counter.clone()))
            .build();

        assert_eq!(mediator.send(Ping).await.unwrap(), "pong");
        assert_eq!(mediator.query(Count).await.unwrap(), 7);
        mediator
            .publish(Tick { at: Utc::now() }, "agg", "T", 1)
            .await
            .unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn missing_handler_errors() {
        let mediator = Mediator::builder().build();
        assert!(mediator.send(Ping).await.is_err());
        assert!(mediator.query(Count).await.is_err());
    }

    #[tokio::test]
    async fn unknown_event_is_noop() {
        let mediator = Mediator::builder().build();
        mediator
            .publish(Tick { at: Utc::now() }, "agg", "T", 0)
            .await
            .unwrap();
    }
}
