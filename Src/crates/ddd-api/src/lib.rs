//! # ddd-api
//!
//! Reusable building blocks for exposing DDD applications via gRPC and REST
//! (HTTP) APIs.
//!
//! ## Feature flags
//!
//! | Feature | Contents |
//! |---------|----------|
//! | `grpc` (default) | gRPC server, interceptors, error mapping, pagination |
//! | `rest` (default) | REST server, middleware, Problem Details (RFC 9457) |
//! | `openapi` (default) | OpenAPI + Scalar UI integration (depends on `rest`) |
//! | `telemetry` | OpenTelemetry tracing interceptors |
//! | `jwt` | JWT bearer-token auth modules under `grpc::auth` and `rest::auth` (propagates `ddd-shared-kernel/jwt`) |
//! | `full` | `grpc` + `rest` + `openapi` + `telemetry` + `jwt` |

#![warn(missing_docs)]

pub mod common;

#[cfg(feature = "grpc")]
pub mod grpc;

#[cfg(feature = "rest")]
pub mod rest;

// ─── Crate-root re-exports ───────────────────────────────────────────────────

#[cfg(feature = "rest")]
pub use rest::{
    catch_panic_layer, fallback_handler, health_router, ApiResponse, FieldViolation, HealthCheck,
    HealthCheckRegistry, IdempotencyKey, PageDto, ProblemDetail, ProblemDetailExt,
    RestErrorResponse, RestServer,
};

#[cfg(feature = "grpc")]
pub use grpc::{
    error_mapping_interceptor, extract_idempotency_key, FromProto, GrpcErrorExt, GrpcServer,
    HasMetadata, IntoProto, TonicStream,
};

/// Curated imports for REST / gRPC adapter modules.
///
/// ```ignore
/// use ddd_api::prelude::*;
/// ```
///
/// Bundles error types, pagination DTOs, and the Problem Details / Status
/// helpers most handlers need. Only the items from enabled features are
/// included, so using the prelude does not itself require any feature.
pub mod prelude {
    pub use ddd_shared_kernel::{AppError, AppResult, Page, PageRequest};

    #[cfg(feature = "rest")]
    pub use crate::rest::{
        FieldViolation, IdempotencyKey, PageDto, ProblemDetail, ProblemDetailExt,
    };

    #[cfg(feature = "grpc")]
    pub use crate::grpc::{extract_idempotency_key, GrpcErrorExt, HasMetadata};
}
