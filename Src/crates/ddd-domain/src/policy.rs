//! The Policy pattern — mutable business rules that may veto an operation.

use std::fmt;

/// A rule violation returned when a [`Policy`] rejects a candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyViolation {
    /// Human-readable description.
    pub message: String,
    /// Optional machine-readable code.
    pub code: Option<String>,
}

impl PolicyViolation {
    /// Create a new violation.
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), code: None }
    }

    /// Attach a machine-readable code.
    pub fn with_code(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self { message: message.into(), code: Some(code.into()) }
    }
}

impl fmt::Display for PolicyViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.code {
            Some(c) => write!(f, "{} [{}]", self.message, c),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for PolicyViolation {}

/// A business rule that inspects — and may mutate — a candidate value.
pub trait Policy<T: ?Sized>: Send + Sync {
    /// Apply the policy; `Ok(())` means the candidate is accepted.
    fn apply(&self, candidate: &mut T) -> Result<(), PolicyViolation>;
}

/// Adapter that lets any `Fn(&mut T) -> Result<(), PolicyViolation>` act as a
/// [`Policy`].
pub struct ClosurePolicy<T: ?Sized, F> {
    f: F,
    _t: std::marker::PhantomData<fn(&mut T)>,
}

impl<T: ?Sized, F> ClosurePolicy<T, F>
where
    F: Fn(&mut T) -> Result<(), PolicyViolation> + Send + Sync,
{
    /// Wrap a closure.
    pub fn new(f: F) -> Self {
        Self { f, _t: std::marker::PhantomData }
    }
}

impl<T, F> Policy<T> for ClosurePolicy<T, F>
where
    T: ?Sized,
    F: Fn(&mut T) -> Result<(), PolicyViolation> + Send + Sync,
{
    fn apply(&self, candidate: &mut T) -> Result<(), PolicyViolation> {
        (self.f)(candidate)
    }
}

/// An ordered chain of policies, applied until one fails.
pub struct PolicyChain<T: ?Sized> {
    policies: Vec<Box<dyn Policy<T>>>,
}

impl<T: ?Sized> Default for PolicyChain<T> {
    fn default() -> Self {
        Self { policies: Vec::new() }
    }
}

impl<T: ?Sized> PolicyChain<T> {
    /// Create an empty chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a policy.
    #[must_use]
    pub fn with<P: Policy<T> + 'static>(mut self, policy: P) -> Self {
        self.policies.push(Box::new(policy));
        self
    }

    /// Append a boxed policy.
    pub fn push(&mut self, policy: Box<dyn Policy<T>>) {
        self.policies.push(policy);
    }

    /// Apply every policy in order, short-circuiting on the first violation.
    pub fn apply(&self, candidate: &mut T) -> Result<(), PolicyViolation> {
        for p in &self.policies {
            p.apply(candidate)?;
        }
        Ok(())
    }

    /// Apply every policy in order, collecting *all* violations.
    pub fn apply_all(&self, candidate: &mut T) -> Result<(), Vec<PolicyViolation>> {
        let mut errs = Vec::new();
        for p in &self.policies {
            if let Err(e) = p.apply(candidate) {
                errs.push(e);
            }
        }
        if errs.is_empty() { Ok(()) } else { Err(errs) }
    }
}

/// Build a [`ClosurePolicy`] from a closure.
///
/// # Example
/// ```
/// use ddd_domain::policy;
/// use ddd_domain::policy::{Policy, PolicyViolation};
///
/// let not_negative = policy!(|n: &mut i32| {
///     if *n < 0 { Err(PolicyViolation::new("must be non-negative")) } else { Ok(()) }
/// });
/// let mut v = 5;
/// assert!(not_negative.apply(&mut v).is_ok());
/// ```
#[macro_export]
macro_rules! policy {
    ($closure:expr) => {
        $crate::policy::ClosurePolicy::new($closure)
    };
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_short_circuits() {
        let chain: PolicyChain<i32> = PolicyChain::new()
            .with(ClosurePolicy::new(|n: &mut i32| {
                if *n < 0 { Err(PolicyViolation::new("negative")) } else { Ok(()) }
            }))
            .with(ClosurePolicy::new(|n: &mut i32| {
                if *n > 100 { Err(PolicyViolation::new("too big")) } else { Ok(()) }
            }));
        let mut v = 5;
        assert!(chain.apply(&mut v).is_ok());
        let mut v = -1;
        assert_eq!(chain.apply(&mut v).unwrap_err().message, "negative");
    }

    #[test]
    fn apply_all_collects() {
        let chain: PolicyChain<i32> = PolicyChain::new()
            .with(ClosurePolicy::new(|_: &mut i32| Err(PolicyViolation::new("a"))))
            .with(ClosurePolicy::new(|_: &mut i32| Err(PolicyViolation::new("b"))));
        let mut v = 0;
        let errs = chain.apply_all(&mut v).unwrap_err();
        assert_eq!(errs.len(), 2);
    }
}
