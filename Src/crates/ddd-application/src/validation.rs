//! Application-layer validation — thin wrappers over
//! [`ddd_shared_kernel::validation`] plus chain / fluent builders.

pub use ddd_shared_kernel::validation::{
    ValidationError, ValidationResult, ValidationRule,
};

/// A validator over values of type `T`.
pub trait Validator<T>: Send + Sync {
    /// Run validation and return the accumulated result.
    fn validate(&self, value: &T) -> ValidationResult;
}

/// Composes multiple validators; their results are merged.
pub struct ValidatorChain<T> {
    validators: Vec<Box<dyn Validator<T>>>,
}

impl<T> Default for ValidatorChain<T> {
    fn default() -> Self {
        Self { validators: Vec::new() }
    }
}

impl<T> ValidatorChain<T> {
    /// Create an empty chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a validator.
    #[must_use]
    pub fn with<V: Validator<T> + 'static>(mut self, v: V) -> Self {
        self.validators.push(Box::new(v));
        self
    }
}

impl<T: Send + Sync> Validator<T> for ValidatorChain<T> {
    fn validate(&self, value: &T) -> ValidationResult {
        let mut acc = ValidationResult::new();
        for v in &self.validators {
            acc.merge(v.validate(value));
        }
        acc
    }
}

/// Fluent builder that accumulates per-field rules against a single value.
pub struct FluentValidator<'a, T> {
    value: &'a T,
    result: ValidationResult,
}

impl<'a, T> FluentValidator<'a, T> {
    /// Start validating `value`.
    pub fn new(value: &'a T) -> Self {
        Self { value, result: ValidationResult::new() }
    }

    /// Run a rule closure against a field projection of `value`.
    #[must_use]
    pub fn for_field<F>(mut self, rule_fn: F) -> Self
    where
        F: FnOnce(&T) -> ValidationResult,
    {
        self.result.merge(rule_fn(self.value));
        self
    }

    /// Finish and return the accumulated result.
    pub fn validate(self) -> ValidationResult {
        self.result
    }
}

/// Build a [`ValidationRule`] for a `value`/`field` pair — identical in shape
/// to [`ddd_shared_kernel::validate!`] but resolved through this crate.
///
/// # Example
/// ```
/// use ddd_application::validate;
/// use ddd_application::validation::ValidationResult;
///
/// let name = "alice";
/// let r: ValidationResult = validate!(name, "name").not_empty().into();
/// assert!(r.is_valid());
/// ```
#[macro_export]
macro_rules! validate {
    ($value:expr, $field:expr) => {
        $crate::validation::ValidationRule::new($value, $field)
    };
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct NonEmpty;
    impl Validator<String> for NonEmpty {
        fn validate(&self, v: &String) -> ValidationResult {
            ValidationRule::new(v.as_str(), "value").not_empty().into()
        }
    }

    #[test]
    fn chain_merges_results() {
        let chain: ValidatorChain<String> = ValidatorChain::new().with(NonEmpty);
        let ok = chain.validate(&"hi".to_owned());
        assert!(ok.is_valid());
        let bad = chain.validate(&String::new());
        assert!(!bad.is_valid());
    }

    #[test]
    fn fluent_validator_accumulates() {
        let v = "".to_owned();
        let r = FluentValidator::new(&v)
            .for_field(|s| {
                ValidationRule::new(s.as_str(), "name").not_empty().into()
            })
            .validate();
        assert!(!r.is_valid());
    }
}
