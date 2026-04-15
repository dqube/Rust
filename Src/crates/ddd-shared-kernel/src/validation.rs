//! Fluent validation API.
//!
//! Build rules with [`ValidationRule<T>`], collect results in
//! [`ValidationResult`], and orchestrate across fields with
//! [`FluentValidator<T>`].
//!
//! # Example
//! ```
//! use ddd_shared_kernel::validation::{ValidationRule, ValidationResult};
//! use ddd_shared_kernel::validate;
//!
//! let name = "Alice";
//! let result: ValidationResult = validate!(name, "name")
//!     .not_empty()
//!     .min_length(2)
//!     .max_length(50)
//!     .into();
//!
//! assert!(result.is_valid());
//! ```

use crate::{AppError, AppResult};

// ─── ValidationError ─────────────────────────────────────────────────────────

/// A single validation failure on one field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Field name.
    pub field: String,
    /// Human-readable message.
    pub message: String,
    /// Machine-readable code (e.g. `"min_length"`).
    pub code: String,
    /// The value that was attempted (as a debug string; may be redacted).
    pub attempted_value: Option<String>,
}

impl ValidationError {
    fn new(field: &str, message: impl Into<String>, code: &str) -> Self {
        Self {
            field: field.to_owned(),
            message: message.into(),
            code: code.to_owned(),
            attempted_value: None,
        }
    }

    fn with_value(mut self, value: &dyn std::fmt::Debug) -> Self {
        self.attempted_value = Some(format!("{value:?}"));
        self
    }
}

// ─── ValidationResult ────────────────────────────────────────────────────────

/// An ordered collection of [`ValidationError`]s from one or more rules.
#[derive(Debug, Default, Clone)]
pub struct ValidationResult {
    errors: Vec<ValidationError>,
}

impl ValidationResult {
    /// Create an empty (valid) result.
    pub fn new() -> Self {
        Self::default()
    }

    /// `true` when no errors have been recorded.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Borrow the recorded errors.
    pub fn errors(&self) -> &[ValidationError] {
        &self.errors
    }

    /// Merge `other` into `self`.
    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
    }

    /// Combine two results, consuming both.
    pub fn and(mut self, other: ValidationResult) -> Self {
        self.errors.extend(other.errors);
        self
    }

    /// Convert to an [`AppError`].
    ///
    /// Returns `Ok(())` when valid, otherwise returns
    /// [`AppError::ValidationBatch`] (or [`AppError::Validation`] for a
    /// single error).
    pub fn into_app_error(self) -> AppResult<()> {
        if self.is_valid() {
            return Ok(());
        }

        if self.errors.len() == 1 {
            let e = &self.errors[0];
            return Err(AppError::validation(&e.field, &e.message));
        }

        use crate::ValidationFieldError;
        let batch = self
            .errors
            .iter()
            .map(|e| ValidationFieldError::with_code(&e.field, &e.message, &e.code))
            .collect();
        Err(AppError::validation_batch(batch))
    }

    /// Convert to a `tonic::Status` (only available with the `grpc` feature).
    #[cfg(feature = "grpc")]
    pub fn to_grpc_status(&self) -> tonic::Status {
        if self.is_valid() {
            return tonic::Status::ok("ok");
        }
        let msg = self
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.field, e.message))
            .collect::<Vec<_>>()
            .join("; ");
        tonic::Status::invalid_argument(msg)
    }
}

// ─── ValidationRule ──────────────────────────────────────────────────────────

/// A chainable rule builder for a single value/field pair.
///
/// Create one via the [`validate!`] macro, chain rule methods, then either
/// convert into a [`ValidationResult`] with `.into()` or call `.finish()`.
pub struct ValidationRule<'a, T: ?Sized> {
    value: &'a T,
    field: &'a str,
    errors: Vec<ValidationError>,
}

impl<'a, T: ?Sized + std::fmt::Debug> ValidationRule<'a, T> {
    /// Create a new rule for `value` associated with `field`.
    pub fn new(value: &'a T, field: &'a str) -> Self {
        Self {
            value,
            field,
            errors: vec![],
        }
    }

    /// Add a custom predicate.
    ///
    /// `predicate` receives the value and returns `true` when the value is
    /// acceptable.  `message` and `code` describe the failure.
    pub fn must(
        mut self,
        predicate: impl Fn(&T) -> bool,
        message: impl Into<String>,
        code: &str,
    ) -> Self {
        if !predicate(self.value) {
            self.errors.push(
                ValidationError::new(self.field, message, code).with_value(&self.value),
            );
        }
        self
    }

    /// Consume the rule and return a [`ValidationResult`].
    pub fn finish(self) -> ValidationResult {
        ValidationResult {
            errors: self.errors,
        }
    }
}

impl<'a, T: ?Sized + std::fmt::Debug> From<ValidationRule<'a, T>> for ValidationResult {
    fn from(rule: ValidationRule<'a, T>) -> ValidationResult {
        rule.finish()
    }
}

// ── String rules ─────────────────────────────────────────────────────────────

impl<'a> ValidationRule<'a, str> {
    /// Value must not be empty (after trimming).
    pub fn not_empty(self) -> Self {
        self.must(|v: &str| !v.trim().is_empty(), "must not be empty", "not_empty")
    }

    /// Value must be at least `min` characters long.
    pub fn min_length(self, min: usize) -> Self {
        self.must(
            move |v: &str| v.chars().count() >= min,
            format!("must be at least {min} characters"),
            "min_length",
        )
    }

    /// Value must be at most `max` characters long.
    pub fn max_length(self, max: usize) -> Self {
        self.must(
            move |v: &str| v.chars().count() <= max,
            format!("must be at most {max} characters"),
            "max_length",
        )
    }

    /// Value must look like an email address (basic check).
    pub fn email(self) -> Self {
        self.must(
            |v: &str| {
                let s = v.trim();
                s.contains('@') && s.contains('.') && s.len() >= 5
            },
            "must be a valid email address",
            "email",
        )
    }

    /// Value must contain at least one non-whitespace character.
    pub fn not_blank(self) -> Self {
        self.not_empty()
    }

    /// Value must be one of the provided options.
    pub fn one_of(self, options: &'a [&'a str]) -> Self {
        self.must(
            move |v: &str| options.contains(&v),
            format!("must be one of: {}", options.join(", ")),
            "one_of",
        )
    }

    /// Value must match a regex pattern (requires `validation` feature).
    #[cfg(feature = "validation")]
    pub fn matches_pattern(self, pattern: &str) -> Self {
        match regex::Regex::new(pattern) {
            Err(_) => self,
            Ok(re) => self.must(
                move |v: &str| re.is_match(v),
                format!("must match pattern '{pattern}'"),
                "matches_pattern",
            ),
        }
    }
}

// ── Numeric rules ─────────────────────────────────────────────────────────────

macro_rules! impl_numeric_rules {
    ($($t:ty),+) => {
        $(
            impl<'a> ValidationRule<'a, $t> {
                /// Value must be greater than zero.
                pub fn positive(self) -> Self {
                    self.must(|v: &$t| *v > (0 as $t), "must be positive", "positive")
                }

                /// Value must be within `[min, max]`.
                pub fn in_range(self, min: $t, max: $t) -> Self {
                    self.must(
                        move |v: &$t| *v >= min && *v <= max,
                        format!("must be between {min} and {max}"),
                        "in_range",
                    )
                }
            }
        )+
    };
}

impl_numeric_rules!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);

// ── Option rules ──────────────────────────────────────────────────────────────

impl<'a, T: std::fmt::Debug> ValidationRule<'a, Option<T>> {
    /// Value must be `Some`.
    pub fn required(self) -> Self {
        self.must(|v: &Option<T>| v.is_some(), "is required", "required")
    }
}

// ─── FluentValidator ─────────────────────────────────────────────────────────

/// Orchestrates multiple field validations against a single value of type `T`.
///
/// # Example
/// ```
/// use ddd_shared_kernel::validation::{FluentValidator, ValidationResult};
///
/// struct CreateUserDto {
///     name: String,
///     email: String,
/// }
///
/// let dto = CreateUserDto {
///     name: "Alice".to_owned(),
///     email: "alice@example.com".to_owned(),
/// };
///
/// let result: ValidationResult = FluentValidator::new(&dto)
///     .field("name",  |d: &CreateUserDto| {
///         use ddd_shared_kernel::validation::ValidationRule;
///         ValidationRule::new(d.name.as_str(), "name").not_empty().into()
///     })
///     .validate();
///
/// assert!(result.is_valid());
/// ```
type RuleFn<'a, T> = Box<dyn Fn(&T) -> ValidationResult + 'a>;

/// Fluent builder that collects validation rules and evaluates them against a
/// borrowed value.
pub struct FluentValidator<'a, T> {
    value: &'a T,
    rules: Vec<RuleFn<'a, T>>,
}

impl<'a, T> FluentValidator<'a, T> {
    /// Create a validator for `value`.
    pub fn new(value: &'a T) -> Self {
        Self {
            value,
            rules: vec![],
        }
    }

    /// Register a field rule.
    ///
    /// `rule_fn` receives the whole value and returns a [`ValidationResult`]
    /// for that field.
    pub fn field<F>(mut self, _field: &'a str, rule_fn: F) -> Self
    where
        F: Fn(&T) -> ValidationResult + 'a,
    {
        self.rules.push(Box::new(rule_fn));
        self
    }

    /// Run all registered rules and return the combined [`ValidationResult`].
    pub fn validate(self) -> ValidationResult {
        let mut result = ValidationResult::new();
        for rule in &self.rules {
            result.merge(rule(self.value));
        }
        result
    }
}

// ─── Macros ──────────────────────────────────────────────────────────────────

/// Start a fluent validation rule for `$value` associated with `$field`.
///
/// # Example
/// ```
/// use ddd_shared_kernel::validate;
/// use ddd_shared_kernel::validation::ValidationResult;
///
/// let email = "test@example.com";
/// let result: ValidationResult = validate!(email, "email")
///     .not_empty()
///     .email()
///     .into();
///
/// assert!(result.is_valid());
/// ```
#[macro_export]
macro_rules! validate {
    ($value:expr, $field:expr) => {
        $crate::validation::ValidationRule::new($value, $field)
    };
}

/// Combine multiple [`ValidationResult`]s into one.
///
/// # Example
/// ```
/// use ddd_shared_kernel::{validate, validate_all};
/// use ddd_shared_kernel::validation::ValidationResult;
///
/// let name = "Alice";
/// let age: i32 = 30;
///
/// let result: ValidationResult = validate_all!(
///     validate!(name, "name").not_empty().into(),
///     validate!(&age, "age").positive().into()
/// );
///
/// assert!(result.is_valid());
/// ```
#[macro_export]
macro_rules! validate_all {
    ($first:expr $(, $rest:expr)*) => {{
        let mut __result: $crate::validation::ValidationResult = $first;
        $(
            __result.merge($rest);
        )*
        __result
    }};
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_string_passes() {
        let r: ValidationResult = ValidationRule::new("hello", "field")
            .not_empty()
            .min_length(3)
            .max_length(10)
            .into();
        assert!(r.is_valid());
    }

    #[test]
    fn empty_string_fails_not_empty() {
        let r: ValidationResult = ValidationRule::new("", "field").not_empty().into();
        assert!(!r.is_valid());
        assert_eq!(r.errors()[0].code, "not_empty");
    }

    #[test]
    fn too_short_fails_min_length() {
        let r: ValidationResult = ValidationRule::new("hi", "name").min_length(5).into();
        assert!(!r.is_valid());
        assert_eq!(r.errors()[0].code, "min_length");
    }

    #[test]
    fn email_validation() {
        let good: ValidationResult = ValidationRule::new("a@b.com", "email").email().into();
        assert!(good.is_valid());

        let bad: ValidationResult = ValidationRule::new("notanemail", "email").email().into();
        assert!(!bad.is_valid());
    }

    #[test]
    fn positive_integer() {
        let r: ValidationResult = ValidationRule::new(&-1_i32, "count").positive().into();
        assert!(!r.is_valid());
    }

    #[test]
    fn in_range_integer() {
        let ok: ValidationResult = ValidationRule::new(&5_u32, "val").in_range(1, 10).into();
        assert!(ok.is_valid());

        let fail: ValidationResult = ValidationRule::new(&11_u32, "val").in_range(1, 10).into();
        assert!(!fail.is_valid());
    }

    #[test]
    fn into_app_error_batch() {
        let r = ValidationResult {
            errors: vec![
                ValidationError::new("a", "bad", "code"),
                ValidationError::new("b", "bad", "code"),
            ],
        };
        let err = r.into_app_error().unwrap_err();
        assert!(err.is_validation());
    }

    #[test]
    fn attempted_value_stored() {
        let r: ValidationResult = ValidationRule::new("", "field").not_empty().into();
        let e = &r.errors()[0];
        // The debug representation of an empty &str is `""`
        assert!(e.attempted_value.is_some());
    }

    #[test]
    fn validate_all_macro() {
        let name = "Alice";
        let age: i32 = 25;
        let result = validate_all!(
            validate!(name, "name").not_empty().into(),
            validate!(&age, "age").positive().into()
        );
        assert!(result.is_valid());
    }

    #[test]
    fn validate_macro_chains() {
        let email = "user@example.com";
        let r: ValidationResult = validate!(email, "email").not_empty().email().into();
        assert!(r.is_valid());
    }

    #[test]
    fn option_required_rule() {
        let none_val: Option<String> = None;
        let r: ValidationResult = ValidationRule::new(&none_val, "field").required().into();
        assert!(!r.is_valid());
        assert_eq!(r.errors()[0].code, "required");
    }

    #[test]
    fn one_of_rule() {
        let val = "active";
        let ok: ValidationResult =
            ValidationRule::new(val, "status").one_of(&["active", "inactive"]).into();
        assert!(ok.is_valid());

        let fail: ValidationResult =
            ValidationRule::new("pending", "status").one_of(&["active", "inactive"]).into();
        assert!(!fail.is_valid());
    }
}
