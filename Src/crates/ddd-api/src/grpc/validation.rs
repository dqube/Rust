//! gRPC request validation integration.
//!
//! Two layers are exposed:
//!
//! - Per-field fluent validation via [`GrpcValidationExt`] and the
//!   [`grpc_validate!`] macro — convenient for ad-hoc checks inline.
//! - Registry-backed typed validation via
//!   [`GrpcValidatorRegistryExt::validate`] and [`grpc_validate_with!`] —
//!   reuses the same [`ValidatorRegistry`] the REST extractor consumes, so
//!   validators are defined once and both transports agree.
//!
//! Failures are returned as `tonic::Status::invalid_argument` **and** carry:
//!
//! 1. An RFC 9457 Problem Details JSON body in the `problem-details-bin`
//!    binary metadata header (same shape as the REST API).
//! 2. A `google.rpc.BadRequest`-compatible `bad-request-bin` binary metadata
//!    entry with structured field violations, so standard gRPC clients can
//!    parse errors without custom deserialization.

use ddd_application::ValidatorRegistry;
use ddd_shared_kernel::validation::{FluentValidator, ValidationResult};
use ddd_shared_kernel::AppError;
use tonic::{metadata::MetadataValue, Status};

use crate::rest::problem_details::{ProblemDetail, ProblemDetailExt};

// ─── Per-field fluent validation ────────────────────────────────────────────

/// Extension trait for validating gRPC request payloads inline.
pub trait GrpcValidationExt: Sized + Clone {
    /// Apply fluent validation rules and return `Result<Self, Status>`. On
    /// failure the `Status` carries a Problem Details JSON body in the
    /// `problem-details` binary metadata header.
    #[allow(clippy::result_large_err)]
    fn validate_grpc(
        self,
        rules: impl FnOnce(FluentValidator<'_, Self>) -> FluentValidator<'_, Self>,
    ) -> Result<Self, Status> {
        let validator = FluentValidator::new(&self);
        let result: ValidationResult = rules(validator).validate();
        match result.into_app_error() {
            Ok(()) => Ok(self),
            Err(err) => Err(app_error_to_status(&err)),
        }
    }
}

impl<T: Sized + Clone> GrpcValidationExt for T {}

// ─── Registry-backed validation ──────────────────────────────────────────────

/// Run the [`ValidatorRegistry`] validator registered for `T`. Returns
/// `Ok(())` when no validator is registered.
pub trait GrpcValidatorRegistryExt {
    /// Validate `value` using the registry.
    #[allow(clippy::result_large_err)]
    fn validate_grpc<T: 'static>(&self, value: &T) -> Result<(), Status>;
}

impl GrpcValidatorRegistryExt for ValidatorRegistry {
    fn validate_grpc<T: 'static>(&self, value: &T) -> Result<(), Status> {
        self.validate(value).map_err(|e| app_error_to_status(&e))
    }
}

// ─── Error mapping ───────────────────────────────────────────────────────────

/// Convert an [`AppError`] into a `tonic::Status` carrying:
///
/// - A Problem Details JSON body in the `problem-details-bin` metadata header.
/// - For validation errors, a `bad-request-bin` metadata header with
///   `google.rpc.BadRequest`-compatible field violations.
pub fn app_error_to_status(err: &AppError) -> Status {
    let pd = err.to_problem_detail();
    let code = match err {
        AppError::Validation { .. } | AppError::ValidationBatch { .. } => {
            tonic::Code::InvalidArgument
        }
        AppError::NotFound { .. } => tonic::Code::NotFound,
        AppError::Conflict { .. } => tonic::Code::Aborted,
        AppError::Unauthorized { .. } => tonic::Code::Unauthenticated,
        AppError::Forbidden { .. } => tonic::Code::PermissionDenied,
        AppError::BusinessRule { .. } => tonic::Code::FailedPrecondition,
        _ => tonic::Code::Internal,
    };
    let mut status = Status::new(code, pd.detail.clone());
    attach_details(&mut status, &pd);
    status
}

/// Serialise full error details into binary metadata headers:
///
/// - `problem-details-bin`: RFC 9457 Problem Details JSON.
/// - `bad-request-bin`: `google.rpc.BadRequest`-style JSON with field
///   violations (only for validation errors).
fn attach_details(status: &mut Status, pd: &ProblemDetail) {
    // Always attach the Problem Details envelope.
    if let Ok(json) = serde_json::to_vec(pd) {
        let meta = status.metadata_mut();
        meta.insert_bin("problem-details-bin", MetadataValue::from_bytes(&json));
    }

    // For validation errors, also attach a BadRequest-compatible payload.
    if let Some(violations) = &pd.errors {
        let bad_request = BadRequest {
            field_violations: violations
                .iter()
                .map(|v| BadRequestFieldViolation {
                    field: v.field.clone(),
                    description: format!("[{}] {}", v.code, v.message),
                })
                .collect(),
        };
        if let Ok(json) = serde_json::to_vec(&bad_request) {
            let meta = status.metadata_mut();
            meta.insert_bin("bad-request-bin", MetadataValue::from_bytes(&json));
        }
    }
}

/// Mirrors `google.rpc.BadRequest`.
#[derive(Debug, serde::Serialize)]
struct BadRequest {
    field_violations: Vec<BadRequestFieldViolation>,
}

/// Mirrors `google.rpc.BadRequest.FieldViolation`.
#[derive(Debug, serde::Serialize)]
struct BadRequestFieldViolation {
    field: String,
    description: String,
}

/// Create a `Status::invalid_argument` for a single field, with a Problem
/// Details body attached.
pub fn validation_error(field: &str, message: &str) -> Status {
    let err = AppError::validation(field, message);
    app_error_to_status(&err)
}

// ─── Macros ──────────────────────────────────────────────────────────────────

/// Validate a single field with fluent rules. Returns `Result<(), Status>`.
///
/// ```ignore
/// grpc_validate!(name, "name", |r| r.not_empty().min_length(2))?;
/// ```
#[macro_export]
macro_rules! grpc_validate {
    ($value:expr, $field:expr, $rules:expr) => {{
        let __result: ddd_shared_kernel::validation::ValidationResult =
            $rules(ddd_shared_kernel::validation::ValidationRule::new($value, $field)).into();
        match __result.into_app_error() {
            Ok(()) => Ok::<(), tonic::Status>(()),
            Err(__err) => Err($crate::grpc::validation::app_error_to_status(&__err)),
        }
    }};
}

/// Run the registered validator for the value's type from a shared
/// [`ValidatorRegistry`]. Returns `Result<(), tonic::Status>`.
///
/// ```ignore
/// let cmd: CreateOrder = FromProto::from_proto(req.into_inner())?;
/// grpc_validate_with!(self.validators, &cmd)?;
/// ```
#[macro_export]
macro_rules! grpc_validate_with {
    ($registry:expr, $value:expr) => {
        <::ddd_application::ValidatorRegistry as $crate::grpc::validation::GrpcValidatorRegistryExt>
            ::validate_grpc(&*$registry, $value)
    };
}
