//! OpenAPI support for BFF gateways.
//!
//! ## Modules
//!
//! - [`api_routes`] — declarative [`ApiRoute`] catalogue and
//!   [`inject_routes`] OpenAPI injection. Always available.
//! - [`router`] — Scalar UI + JSON spec axum router. Requires the
//!   `axum-response` feature.
//! - [`merge`] — downstream spec fetching and merging. Requires the
//!   `axum-response` feature (uses `reqwest`).

pub mod api_routes;
#[cfg(feature = "axum-response")]
pub mod merge;
#[cfg(feature = "axum-response")]
pub mod router;

pub use api_routes::{inject_routes, ApiRoute, Param, ResponseSpec, RouteKind, SchemaRef};
#[cfg(feature = "axum-response")]
pub use merge::merged_openapi;
#[cfg(feature = "axum-response")]
pub use router::openapi_router;
