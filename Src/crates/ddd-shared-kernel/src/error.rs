//! Application-level error types.
//!
//! [`AppError`] is the single error type propagated throughout every layer of
//! the application.  Each variant carries enough information for an API handler
//! to produce a correct HTTP response code and a user-facing message.

use std::collections::HashMap;

use thiserror::Error;

// ─── ValidationFieldError ────────────────────────────────────────────────────

/// A single field-level validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationFieldError {
    /// Name of the field that failed validation.
    pub field: String,
    /// Human-readable explanation of why validation failed.
    pub message: String,
    /// Machine-readable error code (e.g. `"min_length"`).
    pub code: String,
}

impl ValidationFieldError {
    /// Create a new error without an explicit code (code defaults to
    /// `"invalid"`).
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code: "invalid".to_owned(),
        }
    }

    /// Create a new error with an explicit machine-readable code.
    pub fn with_code(
        field: impl Into<String>,
        message: impl Into<String>,
        code: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code: code.into(),
        }
    }
}

impl std::fmt::Display for ValidationFieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} [{}]", self.field, self.message, self.code)
    }
}

// ─── AppError ────────────────────────────────────────────────────────────────

/// The unified error type for every application layer.
#[derive(Debug, Error)]
pub enum AppError {
    /// A single field failed validation.
    #[error("Validation error on '{field}': {message}")]
    Validation {
        /// The field name.
        field: String,
        /// Human-readable explanation.
        message: String,
    },

    /// Multiple fields failed validation simultaneously.
    #[error("Validation errors: {}", format_batch(errors))]
    ValidationBatch {
        /// All field-level failures.
        errors: Vec<ValidationFieldError>,
    },

    /// The requested resource does not exist.
    #[error("Not found: {resource} with id '{id}'")]
    NotFound {
        /// Resource type name (e.g. `"User"`).
        resource: String,
        /// Identifier that was looked up.
        id: String,
    },

    /// The operation would create a duplicate resource.
    #[error("Conflict: {message}")]
    Conflict {
        /// Description of the conflict.
        message: String,
    },

    /// The caller is not authenticated.
    #[error("Unauthorized: {message}")]
    Unauthorized {
        /// Reason for rejection.
        message: String,
    },

    /// The caller is authenticated but not allowed to perform this action.
    #[error("Forbidden: {message}")]
    Forbidden {
        /// Reason for rejection.
        message: String,
    },

    /// A domain invariant was violated.
    #[error("Business rule violation: {message}")]
    BusinessRule {
        /// Description of the violated rule.
        message: String,
    },

    /// An unexpected internal failure.
    #[error("Internal error: {message}")]
    Internal {
        /// Description of what went wrong.
        message: String,
    },

    /// A database operation failed.
    #[error("Database error: {message}")]
    Database {
        /// Low-level error description (sanitised for end users).
        message: String,
    },

    /// Serialization or deserialization failed.
    #[error("Serialization error: {message}")]
    Serialization {
        /// Description of the failure.
        message: String,
    },
}

fn format_batch(errors: &[ValidationFieldError]) -> String {
    errors
        .iter()
        .map(|e| format!("{}: {}", e.field, e.message))
        .collect::<Vec<_>>()
        .join(", ")
}

impl AppError {
    // ── Constructors ──────────────────────────────────────────────────────

    /// Create a single-field validation error.
    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create a multi-field validation error.
    pub fn validation_batch(errors: Vec<ValidationFieldError>) -> Self {
        Self::ValidationBatch { errors }
    }

    /// Create a not-found error.
    pub fn not_found(resource: impl Into<String>, id: impl Into<String>) -> Self {
        Self::NotFound {
            resource: resource.into(),
            id: id.into(),
        }
    }

    /// Create a conflict error.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    /// Create an unauthorized error.
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized {
            message: message.into(),
        }
    }

    /// Create a forbidden error.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden {
            message: message.into(),
        }
    }

    /// Create a business-rule violation error.
    pub fn business_rule(message: impl Into<String>) -> Self {
        Self::BusinessRule {
            message: message.into(),
        }
    }

    /// Create an internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create a database error.
    pub fn database(message: impl Into<String>) -> Self {
        Self::Database {
            message: message.into(),
        }
    }

    /// Create a serialization error.
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    /// Returns `true` when this is any kind of validation error.
    pub fn is_validation(&self) -> bool {
        matches!(self, Self::Validation { .. } | Self::ValidationBatch { .. })
    }

    /// Returns `true` when this is a not-found error.
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }

    /// Collect all field-level messages into a `HashMap<field, messages>`.
    ///
    /// Returns an empty map for error variants that are not validation errors.
    pub fn to_validation_map(&self) -> HashMap<String, Vec<String>> {
        match self {
            Self::Validation { field, message } => {
                let mut map = HashMap::new();
                map.insert(field.clone(), vec![message.clone()]);
                map
            }
            Self::ValidationBatch { errors } => {
                let mut map: HashMap<String, Vec<String>> = HashMap::new();
                for e in errors {
                    map.entry(e.field.clone())
                        .or_default()
                        .push(e.message.clone());
                }
                map
            }
            _ => HashMap::new(),
        }
    }

    /// Suggested HTTP status code for this error.
    pub fn http_status_code(&self) -> u16 {
        match self {
            Self::Validation { .. } | Self::ValidationBatch { .. } => 422,
            Self::NotFound { .. } => 404,
            Self::Conflict { .. } => 409,
            Self::Unauthorized { .. } => 401,
            Self::Forbidden { .. } => 403,
            Self::BusinessRule { .. } => 422,
            Self::Internal { .. } | Self::Database { .. } | Self::Serialization { .. } => 500,
        }
    }
}

/// Convenience alias used throughout all crates.
pub type AppResult<T> = Result<T, AppError>;

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_constructor() {
        let err = AppError::validation("email", "must be a valid email address");
        assert!(err.is_validation());
        let map = err.to_validation_map();
        assert_eq!(map["email"], vec!["must be a valid email address"]);
    }

    #[test]
    fn validation_batch_aggregates_messages() {
        let errs = vec![
            ValidationFieldError::new("name", "cannot be empty"),
            ValidationFieldError::new("name", "too short"),
            ValidationFieldError::with_code("age", "must be positive", "positive"),
        ];
        let err = AppError::validation_batch(errs);
        let map = err.to_validation_map();
        assert_eq!(map["name"].len(), 2);
        assert_eq!(map["age"].len(), 1);
    }

    #[test]
    fn not_found_display() {
        let err = AppError::not_found("User", "abc-123");
        let s = err.to_string();
        assert!(s.contains("User"));
        assert!(s.contains("abc-123"));
    }

    #[test]
    fn http_status_codes() {
        assert_eq!(AppError::not_found("X", "1").http_status_code(), 404);
        assert_eq!(AppError::conflict("dup").http_status_code(), 409);
        assert_eq!(AppError::unauthorized("no token").http_status_code(), 401);
        assert_eq!(AppError::internal("boom").http_status_code(), 500);
    }

    #[test]
    fn validation_field_error_display() {
        let e = ValidationFieldError::with_code("email", "bad format", "email_format");
        let s = e.to_string();
        assert!(s.contains("email"));
        assert!(s.contains("bad format"));
        assert!(s.contains("email_format"));
    }
}
