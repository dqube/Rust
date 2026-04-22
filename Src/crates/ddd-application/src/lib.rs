//! # ddd-application
//!
//! Reusable application-layer building blocks for a DDD / CQRS microservice:
//! CQRS command / query buses, validation helpers, unit-of-work,
//! infrastructure ports (clock, id generator, event publisher), domain-event
//! handler registry, use-case traits, and pagination helpers.
//!
//! This crate provides *generic* primitives — concrete commands, queries,
//! and handlers live in bounded-context crates.

#![warn(missing_docs)]

pub mod cqrs;
pub mod event_handling;
pub mod idempotency;
pub mod macros;
pub mod mediator;
pub mod pagination;
pub mod ports;
pub mod saga;
pub mod testing;
pub mod unit_of_work;
pub mod use_case;
pub mod validation;
pub mod validator_registry;

pub use cqrs::{Command, CommandBus, CommandHandler, Query, QueryBus, QueryHandler};
pub use event_handling::{BoxedDomainEvent, DomainEventHandler, EventHandlerRegistry};
pub use mediator::{HandlerRegistration, Mediator, MediatorBuilder, MediatorRegistry};
pub use validator_registry::{ErasedValidator, ValidatorRegistration, ValidatorRegistry};
pub use pagination::{page_request_from_params, Page, PageRequest};
pub use ports::{Clock, EventPublisher, IdGenerator, NullEventPublisher, SystemClock, UuidV7Generator};
pub use unit_of_work::{UnitOfWork, UnitOfWorkFactory};
pub use use_case::{UseCase, ValidatedUseCase};
pub use validation::{FluentValidator, Validator, ValidatorChain};
pub use idempotency::{IdempotentCommand, IdempotentCommandHandler};
pub use saga::{DefaultSagaOrchestrator, SagaDefinitionRegistry};

/// Curated imports for command/query handler modules.
///
/// ```ignore
/// use ddd_application::prelude::*;
/// ```
///
/// Bundles the `Command`/`Query` traits, their handler traits, the `Mediator`,
/// `UnitOfWork`, and the most common shared-kernel error/pagination types.
/// Use this inside per-service handler modules; avoid in library crates and
/// wiring files where origin clarity matters.
pub mod prelude {
    pub use crate::cqrs::{Command, CommandHandler, Query, QueryHandler};
    pub use crate::mediator::Mediator;
    pub use crate::ports::{Clock, EventPublisher, IdGenerator};
    pub use crate::unit_of_work::UnitOfWork;
    pub use ddd_shared_kernel::{AppError, AppResult, Page, PageRequest};
}
