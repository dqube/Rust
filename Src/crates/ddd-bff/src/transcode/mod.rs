//! Error mapping and RFC 9457 Problem Details for BFF gateways.
//!
//! This module owns the `gRPC Status → AppError → ProblemDetail` pipeline
//! that every REST → gRPC handler needs to surface downstream failures in
//! a machine-readable, HTTP-friendly way.

pub mod errors;

pub use errors::{
    app_error_to_problem, grpc_status_to_app_error, method_not_allowed, route_not_found,
    upstream_unavailable, FieldViolation, GrpcIntoProblem, IntoProblem, ProblemDetail,
    PROBLEM_CONTENT_TYPE,
};
#[cfg(feature = "axum-response")]
pub use errors::fallback_handler;
