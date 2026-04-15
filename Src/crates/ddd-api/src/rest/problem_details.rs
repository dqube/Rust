//! RFC 9457 Problem Details for HTTP APIs.
//!
//! Every [`AppError`] variant is converted into a fully compliant RFC 9457
//! Problem Details response with:
//!
//! - A stable `type` URI per error category for machine-readable triage.
//! - `title`, `status`, `detail` per the specification.
//! - `errors` extension for validation failures carrying `field`, `message`,
//!   and `code` per violation.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ddd_shared_kernel::AppError;
use serde::{Deserialize, Serialize};

/// An RFC 9457 Problem Details object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ProblemDetail {
    /// A URI reference identifying the problem type.
    #[serde(rename = "type")]
    pub problem_type: String,
    /// A short, human-readable summary.
    pub title: String,
    /// The HTTP status code.
    pub status: u16,
    /// A human-readable explanation specific to this occurrence.
    pub detail: String,
    /// A URI identifying the specific occurrence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    /// Structured field-level validation errors (only present for validation
    /// failures).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FieldViolation>>,
}

impl ProblemDetail {
    /// Create a new `ProblemDetail`.
    pub fn new(status: u16, title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            problem_type: "about:blank".to_owned(),
            title: title.into(),
            status,
            detail: detail.into(),
            instance: None,
            errors: None,
        }
    }

    /// Set the problem type URI.
    pub fn with_type(mut self, uri: impl Into<String>) -> Self {
        self.problem_type = uri.into();
        self
    }

    /// Set the instance URI.
    pub fn with_instance(mut self, uri: impl Into<String>) -> Self {
        self.instance = Some(uri.into());
        self
    }

    /// Add structured field violations (for validation errors).
    pub fn with_errors(mut self, errors: Vec<FieldViolation>) -> Self {
        self.errors = Some(errors);
        self
    }
}

/// A single field-level validation violation.
///
/// Mirrors `google.rpc.BadRequest.FieldViolation` and the `errors` extension
/// commonly used in RFC 9457 responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct FieldViolation {
    /// The field path that failed validation (e.g. `"email"`,
    /// `"address.zip_code"`).
    pub field: String,
    /// Human-readable description of the violation.
    pub message: String,
    /// Machine-readable error code (e.g. `"min_length"`, `"required"`).
    pub code: String,
}

impl IntoResponse for ProblemDetail {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = serde_json::to_string(&self).unwrap_or_default();
        (
            status,
            [(
                axum::http::header::CONTENT_TYPE,
                "application/problem+json",
            )],
            body,
        )
            .into_response()
    }
}

/// Extension trait for converting an error into a [`ProblemDetail`].
pub trait ProblemDetailExt {
    /// Build a [`ProblemDetail`] from this error.
    fn to_problem_detail(&self) -> ProblemDetail;
}

impl ProblemDetailExt for AppError {
    fn to_problem_detail(&self) -> ProblemDetail {
        let status = self.http_status_code();
        let mut pd = ProblemDetail::new(status, status_title(status), self.to_string())
            .with_type(problem_type_uri(self));

        match self {
            AppError::Validation { field, message } => {
                pd.errors = Some(vec![FieldViolation {
                    field: field.clone(),
                    message: message.clone(),
                    code: "invalid".to_owned(),
                }]);
            }
            AppError::ValidationBatch { errors } => {
                pd.errors = Some(
                    errors
                        .iter()
                        .map(|e| FieldViolation {
                            field: e.field.clone(),
                            message: e.message.clone(),
                            code: e.code.clone(),
                        })
                        .collect(),
                );
            }
            AppError::Database { .. } | AppError::Serialization { .. } => {
                // Don't leak internal details.
                pd.detail = "An internal error occurred".to_owned();
            }
            _ => {}
        }
        pd
    }
}

/// Stable URI per error category, following the RFC 9457 recommendation to use
/// `type` as the primary machine-readable discriminator.
fn problem_type_uri(err: &AppError) -> String {
    let suffix = match err {
        AppError::Validation { .. } | AppError::ValidationBatch { .. } => "validation-error",
        AppError::NotFound { .. } => "not-found",
        AppError::Conflict { .. } => "conflict",
        AppError::Unauthorized { .. } => "unauthorized",
        AppError::Forbidden { .. } => "forbidden",
        AppError::BusinessRule { .. } => "business-rule-violation",
        AppError::Internal { .. } => "internal-error",
        AppError::Database { .. } => "internal-error",
        AppError::Serialization { .. } => "internal-error",
    };
    format!("urn:problem-type:{suffix}")
}

fn status_title(code: u16) -> &'static str {
    crate::common::error_mapping::http_status_title(code)
}

/// Evaluate an expression returning `AppResult<T>` and map errors to
/// [`ProblemDetail`].
///
/// # Example
/// ```ignore
/// use ddd_api::rest_result;
///
/// async fn handler() -> Result<Json<MyDto>, ProblemDetail> {
///     let result = rest_result!(my_use_case.execute(input).await);
///     Ok(Json(result))
/// }
/// ```
#[macro_export]
macro_rules! rest_result {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(err) => {
                return Err(
                    <ddd_shared_kernel::AppError as $crate::rest::ProblemDetailExt>::to_problem_detail(&err),
                )
            }
        }
    };
}
