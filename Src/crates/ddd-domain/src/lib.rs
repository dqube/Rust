//! # ddd-domain
//!
//! Reusable DDD building blocks on top of [`ddd_shared_kernel`]: aggregate and
//! entity helpers, generic repository traits, domain services, the
//! Specification and Policy patterns, and domain-event routing utilities.
//!
//! This crate ships only *generic* primitives — concrete aggregates, value
//! objects, and repositories belong in your bounded-context crates.

#![warn(missing_docs)]

pub mod aggregate;
pub mod domain_service;
pub mod entity;
pub mod error;
pub mod event;
pub mod macros;
pub mod policy;
pub mod repository;
pub mod specification;

pub use aggregate::Aggregate;
pub use domain_service::{DomainService, DomainServiceFor};
pub use entity::Entity;
pub use error::DomainError;
pub use event::{EventPublisher, EventRouter};
pub use policy::{Policy, PolicyChain, PolicyViolation};
pub use repository::Repository;
pub use specification::{AndSpec, ClosureSpec, NotSpec, OrSpec, Specification, SpecificationExt};
