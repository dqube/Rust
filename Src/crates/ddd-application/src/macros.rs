//! Macro index for the crate.
//!
//! All macros live at the crate root via `#[macro_export]`; this module is a
//! documentation-only roll-up.
//!
//! | Macro | Purpose |
//! |-------|---------|
//! | [`impl_command!`](crate::impl_command) | Implement `Command` for a struct |
//! | [`impl_query!`](crate::impl_query) | Implement `Query` for a struct |
//! | [`validate!`](crate::validate) | Start a `ValidationRule` chain |
//! | [`transactional!`](crate::transactional) | Run work inside a UoW |
//! | [`register_handler!`](crate::register_handler) | Register a domain-event handler |
//! | [`impl_use_case!`](crate::impl_use_case) | Implement `UseCase` via closure |
