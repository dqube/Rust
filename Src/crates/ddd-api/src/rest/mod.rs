//! REST / HTTP building blocks: server, middleware, Problem Details, pagination,
//! validation, and OpenAPI integration.

pub mod error;
pub mod global_error_handler;
pub mod health;
pub mod idempotency;
pub mod middleware;
pub mod pagination;
pub mod problem_details;
pub mod server;
pub mod validation;

#[cfg(feature = "openapi")]
pub mod openapi;

#[cfg(feature = "jwt")]
pub mod auth;

pub use error::RestErrorResponse;
pub use global_error_handler::{
    catch_panic_layer, fallback_handler, install, status_to_problem_detail, PanicResponseMapper,
};
pub use health::{health_router, CheckResult, HealthCheck, HealthCheckRegistry, HealthResponse};
pub use idempotency::IdempotencyKey;
pub use pagination::{ApiResponse, PageDto};
pub use problem_details::{FieldViolation, ProblemDetail, ProblemDetailExt};
pub use server::RestServer;
pub use validation::{RestValidator, Validated, ValidatedByRegistry, ValidatorRegistryExt};
