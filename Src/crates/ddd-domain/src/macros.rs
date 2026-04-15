//! Convenience re-exports of every macro defined in this crate.
//!
//! All macros are declared with `#[macro_export]`, so they already live at the
//! crate root (e.g. `ddd_domain::spec!`). This module exists so consumers can
//! write `use ddd_domain::macros::*;` as an alternative import style — it
//! documents the full macro surface in one place.
//!
//! | Macro | Purpose |
//! |-------|---------|
//! | [`define_aggregate!`](crate::define_aggregate) | Generate an aggregate struct |
//! | [`impl_aggregate!`](crate::impl_aggregate) | Implement `AggregateRoot` |
//! | [`impl_aggregate_events!`](crate::impl_aggregate_events) | Add event helper methods |
//! | [`define_repository!`](crate::define_repository) | Generate a repository trait |
//! | [`define_domain_service!`](crate::define_domain_service) | Generate a domain service |
//! | [`define_entity!`](crate::define_entity) | Generate a non-aggregate entity |
//! | [`spec!`](crate::spec) | Build a specification from a closure |
//! | [`combine_specs!`](crate::combine_specs) | Combine specifications |
//! | [`policy!`](crate::policy) | Build a policy from a closure |
//! | [`register_event_handler!`](crate::register_event_handler) | Register a router handler |
