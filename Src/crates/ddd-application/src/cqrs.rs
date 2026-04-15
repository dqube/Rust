//! CQRS building blocks — [`Command`] / [`Query`] traits, handler traits, and
//! in-process [`CommandBus`] / [`QueryBus`] dispatchers.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use ddd_shared_kernel::{AppError, AppResult};

// ─── Command / Query marker traits ───────────────────────────────────────────

/// A command: a request that mutates state.
pub trait Command: Send + Sync + 'static {
    /// The value returned on success.
    type Response: Send + Sync + 'static;
}

/// A query: a request that reads state.
pub trait Query: Send + Sync + 'static {
    /// The value returned on success.
    type Response: Send + Sync + 'static;
}

// ─── Handler traits ──────────────────────────────────────────────────────────

/// Handler for a specific command type.
#[async_trait]
pub trait CommandHandler<C: Command>: Send + Sync {
    /// Execute the command.
    async fn handle(&self, command: C) -> AppResult<C::Response>;
}

/// Handler for a specific query type.
#[async_trait]
pub trait QueryHandler<Q: Query>: Send + Sync {
    /// Execute the query.
    async fn handle(&self, query: Q) -> AppResult<Q::Response>;
}

// ─── CommandBus ──────────────────────────────────────────────────────────────

/// In-process dispatcher that routes commands to their registered handler.
#[derive(Default)]
pub struct CommandBus {
    handlers: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl CommandBus {
    /// Create an empty bus.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler for command type `C`. Replaces any previous handler
    /// for the same type.
    pub fn register<C, H>(&mut self, handler: H)
    where
        C: Command,
        H: CommandHandler<C> + 'static,
    {
        let arc: Arc<dyn CommandHandler<C>> = Arc::new(handler);
        self.handlers.insert(TypeId::of::<C>(), Arc::new(arc));
    }

    /// Dispatch a command to its handler.
    ///
    /// # Errors
    /// Returns [`AppError::Internal`] if no handler is registered for `C`.
    pub async fn dispatch<C: Command>(&self, command: C) -> AppResult<C::Response> {
        let slot = self.handlers.get(&TypeId::of::<C>()).ok_or_else(|| {
            AppError::internal(format!(
                "no command handler registered for {}",
                std::any::type_name::<C>()
            ))
        })?;
        let handler = slot
            .downcast_ref::<Arc<dyn CommandHandler<C>>>()
            .ok_or_else(|| AppError::internal("command handler type mismatch"))?
            .clone();
        handler.handle(command).await
    }
}

// ─── QueryBus ────────────────────────────────────────────────────────────────

/// In-process dispatcher that routes queries to their registered handler.
#[derive(Default)]
pub struct QueryBus {
    handlers: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl QueryBus {
    /// Create an empty bus.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler for query type `Q`.
    pub fn register<Q, H>(&mut self, handler: H)
    where
        Q: Query,
        H: QueryHandler<Q> + 'static,
    {
        let arc: Arc<dyn QueryHandler<Q>> = Arc::new(handler);
        self.handlers.insert(TypeId::of::<Q>(), Arc::new(arc));
    }

    /// Dispatch a query to its handler.
    pub async fn dispatch<Q: Query>(&self, query: Q) -> AppResult<Q::Response> {
        let slot = self.handlers.get(&TypeId::of::<Q>()).ok_or_else(|| {
            AppError::internal(format!(
                "no query handler registered for {}",
                std::any::type_name::<Q>()
            ))
        })?;
        let handler = slot
            .downcast_ref::<Arc<dyn QueryHandler<Q>>>()
            .ok_or_else(|| AppError::internal("query handler type mismatch"))?
            .clone();
        handler.handle(query).await
    }
}

// ─── Macros ──────────────────────────────────────────────────────────────────

/// Implement [`Command`] for an existing struct with the given response type.
///
/// # Example
/// ```
/// use ddd_application::{impl_command, cqrs::Command};
/// pub struct CreateProduct { pub name: String }
/// impl_command!(CreateProduct, String);
/// ```
#[macro_export]
macro_rules! impl_command {
    ($cmd:ty, $resp:ty) => {
        impl $crate::cqrs::Command for $cmd {
            type Response = $resp;
        }
    };
}

/// Implement [`Query`] for an existing struct with the given response type.
#[macro_export]
macro_rules! impl_query {
    ($query:ty, $resp:ty) => {
        impl $crate::cqrs::Query for $query {
            type Response = $resp;
        }
    };
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
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

    #[tokio::test]
    async fn command_bus_dispatches() {
        let mut bus = CommandBus::new();
        bus.register::<Ping, _>(PingHandler);
        let r = bus.dispatch(Ping).await.unwrap();
        assert_eq!(r, "pong");
    }

    #[tokio::test]
    async fn command_bus_missing_handler() {
        let bus = CommandBus::new();
        let r = bus.dispatch(Ping).await;
        assert!(r.is_err());
    }

    struct Count;
    impl Query for Count {
        type Response = u32;
    }
    struct CountHandler;
    #[async_trait]
    impl QueryHandler<Count> for CountHandler {
        async fn handle(&self, _: Count) -> AppResult<u32> {
            Ok(42)
        }
    }

    #[tokio::test]
    async fn query_bus_dispatches() {
        let mut bus = QueryBus::new();
        bus.register::<Count, _>(CountHandler);
        assert_eq!(bus.dispatch(Count).await.unwrap(), 42);
    }
}
