//! BFF middleware — redaction utilities used by the edge observability
//! pipeline, and an axum observability middleware (requires `axum-response`).

pub mod redaction;
#[cfg(feature = "axum-response")]
pub mod axum_observability;
#[cfg(feature = "axum-response")]
pub mod tracing_interceptor;
#[cfg(feature = "axum-response")]
pub mod audit;
#[cfg(feature = "jwt")]
pub mod axum_auth;
