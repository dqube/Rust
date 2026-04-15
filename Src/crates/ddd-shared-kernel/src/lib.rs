//! # shared-kernel
//!
//! Zero-dependency base types and utilities for Domain-Driven Design
//! applications.
//!
//! ## Module overview
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`id`] | [`TypedId<T>`] and [`declare_id!`] |
//! | [`aggregate`] | [`AggregateRoot`] trait, [`impl_aggregate_root!`], [`record_event!`] |
//! | [`domain_event`] | [`DomainEvent`], [`DomainEventEnvelope`], [`DomainEventDispatcher`] |
//! | [`integration_event`] | [`IntegrationEvent`], [`IntegrationEventEnvelope`], [`IntegrationEventPublisher`] |
//! | [`entity`] | [`Entity`] and [`ValueObject`] marker traits |
//! | [`value_object`] | [`impl_value_object!`] macro helper |
//! | [`error`] | [`AppError`], [`AppResult`], [`ValidationFieldError`] |
//! | [`pagination`] | [`Page<T>`] and [`PageRequest`] |
//! | [`outbox`] | [`OutboxMessage`], [`OutboxRepository`], [`OutboxRelay`] |
//! | [`inbox`] | [`InboxMessage`], [`InboxRepository`], [`InboxProcessor`] |
//! | [`dead_letter`] | [`DeadLetterMessage`], [`DeadLetterRepository`], [`DeadLetterAlert`] |
//! | [`idempotency`] | [`IdempotencyStore`], [`IdempotencyRecord`] |
//! | [`saga`] | [`SagaInstance`], [`SagaDefinition`], [`SagaOrchestrator`] |
//! | [`validation`] | Fluent validation API, [`validate!`], [`validate_all!`] |
//! | [`config_validation`] | Bootstrap-time config validator (`Report`, `Validate`); YAML loaders gated behind the `config-validation` feature |
//! | `jwt` (feature `jwt`) | JWT bearer-token validator |
//!
//! ## Feature flags
//!
//! | Feature | Enables |
//! |---------|---------|
//! | `validation` | Pulls `validator` + `regex` for the derive-based validator integration |
//! | `grpc` | Re-exports `tonic::Status` mappings from [`error`] |
//! | `jwt` | Compiles the `jwt` module |
//! | `config-validation` | YAML loader helpers in [`config_validation`] |

#![warn(missing_docs)]

// ─── Modules ─────────────────────────────────────────────────────────────────

pub mod aggregate;
pub mod dead_letter;
pub mod domain_event;
pub mod entity;
pub mod error;
pub mod id;
pub mod idempotency;
pub mod inbox;
pub mod integration_event;
pub mod outbox;
pub mod pagination;
pub mod saga;
pub mod validation;
pub mod value_object;

#[cfg(feature = "jwt")]
pub mod jwt;

pub mod config_validation;

// ─── Re-exports ───────────────────────────────────────────────────────────────

// id
pub use id::TypedId;

// aggregate
pub use aggregate::AggregateRoot;

// domain event
pub use domain_event::{DomainEvent, DomainEventDispatcher, DomainEventEnvelope};

// integration event
pub use integration_event::{
    IntegrationEvent, IntegrationEventEnvelope, IntegrationEventPublisher,
};

// entity / value object
pub use entity::{Entity, ValueObject};

// error
pub use error::{AppError, AppResult, ValidationFieldError};

// pagination
pub use pagination::{Page, PageRequest};

// outbox
pub use outbox::{OutboxMessage, OutboxRelay, OutboxRepository};

// inbox
pub use inbox::{InboxMessage, InboxMessageHandler, InboxProcessor, InboxRepository};

// dead letter
pub use dead_letter::{
    DeadLetterAlert, DeadLetterMessage, DeadLetterOrigin, DeadLetterRepository,
    LogDeadLetterAlert,
};

// idempotency
pub use idempotency::{IdempotencyRecord, IdempotencyStore};

// saga
pub use saga::{
    SagaDefinition, SagaInstance, SagaInstanceRepository, SagaOrchestrator, SagaStatus,
    SagaStepDefinition, SagaStepState, SagaStepStatus,
};

// validation
pub use validation::{FluentValidator, ValidationError, ValidationResult, ValidationRule};

// ─── Macro re-exports ─────────────────────────────────────────────────────────

// These macros are defined with #[macro_export] so they live at the crate root
// and are accessible as `ddd_shared_kernel::declare_id!` etc. without an explicit
// `use` of the originating module.
