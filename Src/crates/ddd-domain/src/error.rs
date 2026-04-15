//! [`DomainError`] — a thin newtype over [`ddd_shared_kernel::AppError`].

use std::fmt;

use ddd_shared_kernel::AppError;

/// A domain-layer error. Wraps [`AppError`] so domain code can construct and
/// propagate errors without depending on every constructor directly.
#[derive(Debug)]
pub struct DomainError(pub AppError);

impl DomainError {
    /// Create a business-rule / domain-invariant violation.
    pub fn domain(message: impl Into<String>) -> Self {
        Self(AppError::business_rule(message))
    }

    /// Create a single-field validation error.
    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self(AppError::validation(field, message))
    }

    /// Create a not-found error.
    pub fn not_found(resource: impl Into<String>, id: impl Into<String>) -> Self {
        Self(AppError::not_found(resource, id))
    }

    /// Create a conflict error.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self(AppError::conflict(message))
    }

    /// Borrow the underlying [`AppError`].
    pub fn as_app_error(&self) -> &AppError {
        &self.0
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for DomainError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl From<DomainError> for AppError {
    fn from(e: DomainError) -> Self {
        e.0
    }
}

impl From<AppError> for DomainError {
    fn from(e: AppError) -> Self {
        Self(e)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_to_app_error() {
        let e: AppError = DomainError::domain("nope").into();
        assert!(matches!(e, AppError::BusinessRule { .. }));
    }

    #[test]
    fn validation_constructor() {
        let e = DomainError::validation("name", "required");
        assert!(e.as_app_error().is_validation());
    }
}
