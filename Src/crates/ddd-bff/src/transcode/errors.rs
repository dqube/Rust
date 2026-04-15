//! Error mapping: gRPC `Status` → [`AppError`], [`AppError`] → RFC 9457
//! [`ProblemDetail`].
//!
//! Transport-free: produces `(status_code, content_type, body_bytes)` — the
//! edge and pingora hooks render it onto whatever response type they own.

use ddd_shared_kernel::AppError;
use serde::Serialize;

/// RFC 9457 Problem Details payload.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "axum-response", derive(utoipa::ToSchema))]
pub struct ProblemDetail {
    /// A URI reference that identifies the problem type.
    #[serde(rename = "type")]
    pub problem_type: String,
    /// Short, human-readable summary of the problem.
    pub title: String,
    /// HTTP status code.
    pub status: u16,
    /// Human-readable explanation specific to this occurrence.
    pub detail: String,
    /// URI identifying the specific occurrence of the problem.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    /// Field-level validation errors, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FieldViolation>>,
}

/// A single field-level validation violation.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "axum-response", derive(utoipa::ToSchema))]
pub struct FieldViolation {
    /// Path to the field (e.g. `items[0].sku`).
    pub field: String,
    /// Human-readable violation message.
    pub message: String,
    /// Machine-friendly code (`invalid`, `required`, …).
    pub code: String,
}

impl ProblemDetail {
    /// Construct a bare [`ProblemDetail`] with `type: about:blank`.
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

    /// Set the `type` URI (e.g. `urn:problem-type:not-found`).
    pub fn with_type(mut self, uri: impl Into<String>) -> Self {
        self.problem_type = uri.into();
        self
    }

    /// Serialise to `application/problem+json` bytes.
    pub fn to_body(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_else(|_| Vec::new())
    }
}

/// MIME type for problem detail responses.
pub const PROBLEM_CONTENT_TYPE: &str = "application/problem+json";

// ─── axum IntoResponse ───────────────────────────────────────────────────────

/// Render a [`ProblemDetail`] as an axum [`Response`] with
/// `Content-Type: application/problem+json`.
#[cfg(feature = "axum-response")]
impl axum::response::IntoResponse for ProblemDetail {
    fn into_response(self) -> axum::response::Response {
        let status = axum::http::StatusCode::from_u16(self.status)
            .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        let body = serde_json::to_string(&self).unwrap_or_default();
        (
            status,
            [(axum::http::header::CONTENT_TYPE, PROBLEM_CONTENT_TYPE)],
            body,
        )
            .into_response()
    }
}

// ─── IntoProblem / GrpcIntoProblem ───────────────────────────────────────────

/// Extension trait for ergonomic `AppResult<T>` → `Result<T, ProblemDetail>`
/// conversion in axum handler return positions.
#[allow(clippy::result_large_err)]
pub trait IntoProblem<T> {
    fn into_problem(self) -> Result<T, ProblemDetail>;
}

impl<T> IntoProblem<T> for ddd_shared_kernel::AppResult<T> {
    #[allow(clippy::result_large_err)]
    fn into_problem(self) -> Result<T, ProblemDetail> {
        self.map_err(|e| app_error_to_problem(&e))
    }
}

/// Extension trait for `Result<T, tonic::Status>` → `Result<T, ProblemDetail>`
/// conversion in axum handler return positions.
#[allow(clippy::result_large_err)]
pub trait GrpcIntoProblem<T> {
    fn into_problem(self) -> Result<T, ProblemDetail>;
}

impl<T> GrpcIntoProblem<T> for Result<T, tonic::Status> {
    #[allow(clippy::result_large_err)]
    fn into_problem(self) -> Result<T, ProblemDetail> {
        self.map_err(|s| app_error_to_problem(&grpc_status_to_app_error(s)))
    }
}

// ─── gRPC Status → AppError ─────────────────────────────────────────────────

/// Convert a [`tonic::Status`] into an [`AppError`].
pub fn grpc_status_to_app_error(status: tonic::Status) -> AppError {
    match status.code() {
        tonic::Code::NotFound => AppError::not_found("resource", status.message()),
        tonic::Code::AlreadyExists => AppError::conflict(status.message()),
        tonic::Code::InvalidArgument => AppError::validation("request", status.message()),
        tonic::Code::Unauthenticated => AppError::unauthorized(status.message()),
        tonic::Code::PermissionDenied => AppError::forbidden(status.message()),
        tonic::Code::FailedPrecondition => AppError::business_rule(status.message()),
        tonic::Code::Unavailable => {
            AppError::internal(format!("service unavailable: {}", status.message()))
        }
        tonic::Code::DeadlineExceeded => {
            AppError::internal(format!("deadline exceeded: {}", status.message()))
        }
        _ => AppError::internal(status.message()),
    }
}

// ─── AppError → ProblemDetail ────────────────────────────────────────────────

/// Convert an [`AppError`] into an RFC 9457 [`ProblemDetail`].
pub fn app_error_to_problem(err: &AppError) -> ProblemDetail {
    let status = err.http_status_code();
    let mut pd = ProblemDetail::new(status, status_title(status), err.to_string())
        .with_type(problem_type_uri(err));

    match err {
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
            pd.detail = "An internal error occurred".to_owned();
        }
        _ => {}
    }

    pd
}

fn problem_type_uri(err: &AppError) -> String {
    let suffix = match err {
        AppError::Validation { .. } | AppError::ValidationBatch { .. } => "validation-error",
        AppError::NotFound { .. } => "not-found",
        AppError::Conflict { .. } => "conflict",
        AppError::Unauthorized { .. } => "unauthorized",
        AppError::Forbidden { .. } => "forbidden",
        AppError::BusinessRule { .. } => "business-rule-violation",
        AppError::Internal { .. } | AppError::Database { .. } | AppError::Serialization { .. } => {
            "internal-error"
        }
    };
    format!("urn:problem-type:{suffix}")
}

fn status_title(code: u16) -> &'static str {
    match code {
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        422 => "Unprocessable Entity",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Error",
    }
}

/// Build a 404 Problem Details for "no matching route".
pub fn route_not_found(method: &str, path: &str) -> ProblemDetail {
    ProblemDetail::new(
        404,
        "Not Found",
        format!("No route matched {method} {path}"),
    )
    .with_type("urn:problem-type:not-found")
}

/// Build a 405 Problem Details listing the allowed methods.
pub fn method_not_allowed(method: &str, allowed: &[String]) -> ProblemDetail {
    ProblemDetail::new(
        405,
        "Method Not Allowed",
        format!(
            "Method {method} not allowed; allowed methods: {}",
            allowed.join(", ")
        ),
    )
    .with_type("urn:problem-type:method-not-allowed")
}

/// Build a 502 Problem Details for upstream connection failures.
pub fn upstream_unavailable(detail: impl Into<String>) -> ProblemDetail {
    ProblemDetail::new(502, "Bad Gateway", detail)
        .with_type("urn:problem-type:upstream-unavailable")
}

/// Axum fallback handler: returns a 404 [`ProblemDetail`] for unmatched routes.
///
/// Wire it with `.fallback(fallback_handler)` on your `axum::Router`.
#[cfg(feature = "axum-response")]
pub async fn fallback_handler() -> ProblemDetail {
    route_not_found("unknown", "unknown")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_grpc_not_found() {
        let err = grpc_status_to_app_error(tonic::Status::not_found("missing"));
        assert!(matches!(err, AppError::NotFound { .. }));
    }

    #[test]
    fn validation_produces_field_violations() {
        let err = AppError::validation("sku", "must not be empty");
        let pd = app_error_to_problem(&err);
        assert!(pd.status == 400 || pd.status == 422);
        let errors = pd.errors.expect("violations");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "sku");
    }

    #[test]
    fn internal_error_has_generic_detail() {
        let err = AppError::Database {
            message: "leaked internals".to_owned(),
        };
        let pd = app_error_to_problem(&err);
        assert_eq!(pd.detail, "An internal error occurred");
        assert_eq!(pd.status, 500);
    }

    #[test]
    fn problem_detail_serialises_json() {
        let pd = route_not_found("GET", "/missing");
        let bytes = pd.to_body();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.contains("\"type\":\"urn:problem-type:not-found\""));
        assert!(text.contains("\"status\":404"));
    }
}

// ─── axum fallback handler ───────────────────────────────────────────────────

