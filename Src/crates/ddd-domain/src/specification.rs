//! The Specification pattern — composable predicates for domain objects.

use std::marker::PhantomData;

/// A predicate over values of type `T`.
pub trait Specification<T: ?Sized>: Send + Sync {
    /// `true` when `candidate` satisfies the specification.
    fn is_satisfied_by(&self, candidate: &T) -> bool;
}

// ── Combinators ──────────────────────────────────────────────────────────────

/// Conjunction of two specifications.
pub struct AndSpec<T: ?Sized, A, B> {
    a: A,
    b: B,
    _t: PhantomData<fn(&T)>,
}

impl<T: ?Sized, A, B> AndSpec<T, A, B> {
    /// Combine `a` AND `b`.
    pub fn new(a: A, b: B) -> Self {
        Self { a, b, _t: PhantomData }
    }
}

impl<T, A, B> Specification<T> for AndSpec<T, A, B>
where
    T: ?Sized,
    A: Specification<T>,
    B: Specification<T>,
{
    fn is_satisfied_by(&self, c: &T) -> bool {
        self.a.is_satisfied_by(c) && self.b.is_satisfied_by(c)
    }
}

/// Disjunction of two specifications.
pub struct OrSpec<T: ?Sized, A, B> {
    a: A,
    b: B,
    _t: PhantomData<fn(&T)>,
}

impl<T: ?Sized, A, B> OrSpec<T, A, B> {
    /// Combine `a` OR `b`.
    pub fn new(a: A, b: B) -> Self {
        Self { a, b, _t: PhantomData }
    }
}

impl<T, A, B> Specification<T> for OrSpec<T, A, B>
where
    T: ?Sized,
    A: Specification<T>,
    B: Specification<T>,
{
    fn is_satisfied_by(&self, c: &T) -> bool {
        self.a.is_satisfied_by(c) || self.b.is_satisfied_by(c)
    }
}

/// Negation of a specification.
pub struct NotSpec<T: ?Sized, A> {
    a: A,
    _t: PhantomData<fn(&T)>,
}

impl<T: ?Sized, A> NotSpec<T, A> {
    /// Negate `a`.
    pub fn new(a: A) -> Self {
        Self { a, _t: PhantomData }
    }
}

impl<T, A> Specification<T> for NotSpec<T, A>
where
    T: ?Sized,
    A: Specification<T>,
{
    fn is_satisfied_by(&self, c: &T) -> bool {
        !self.a.is_satisfied_by(c)
    }
}

// ── Closure specification ────────────────────────────────────────────────────

/// Adapter that lets any `Fn(&T) -> bool` act as a [`Specification`].
pub struct ClosureSpec<T: ?Sized, F> {
    f: F,
    _t: PhantomData<fn(&T)>,
}

impl<T: ?Sized, F> ClosureSpec<T, F>
where
    F: Fn(&T) -> bool + Send + Sync,
{
    /// Wrap a predicate closure.
    pub fn new(f: F) -> Self {
        Self { f, _t: PhantomData }
    }
}

impl<T, F> Specification<T> for ClosureSpec<T, F>
where
    T: ?Sized,
    F: Fn(&T) -> bool + Send + Sync,
{
    fn is_satisfied_by(&self, c: &T) -> bool {
        (self.f)(c)
    }
}

// ── Fluent extension ─────────────────────────────────────────────────────────

/// Fluent combinator methods for any [`Specification`].
pub trait SpecificationExt<T: ?Sized>: Specification<T> + Sized {
    /// Combine with another specification using AND.
    fn and<B: Specification<T>>(self, other: B) -> AndSpec<T, Self, B> {
        AndSpec::new(self, other)
    }

    /// Combine with another specification using OR.
    fn or<B: Specification<T>>(self, other: B) -> OrSpec<T, Self, B> {
        OrSpec::new(self, other)
    }

    /// Negate this specification.
    fn not(self) -> NotSpec<T, Self> {
        NotSpec::new(self)
    }
}

impl<T: ?Sized, S: Specification<T>> SpecificationExt<T> for S {}

// ── Macros ───────────────────────────────────────────────────────────────────

/// Build a [`ClosureSpec`] from a closure.
///
/// # Example
/// ```
/// use ddd_domain::spec;
/// use ddd_domain::specification::Specification;
///
/// let positive = spec!(|n: &i32| *n > 0);
/// assert!(positive.is_satisfied_by(&5));
/// ```
#[macro_export]
macro_rules! spec {
    ($closure:expr) => {
        $crate::specification::ClosureSpec::new($closure)
    };
}

/// Combine multiple specifications with AND (default) or OR.
///
/// # Example
/// ```
/// use ddd_domain::{spec, combine_specs};
/// use ddd_domain::specification::Specification;
///
/// let s = combine_specs!(and: spec!(|n: &i32| *n > 0), spec!(|n: &i32| *n < 10));
/// assert!(s.is_satisfied_by(&5));
/// assert!(!s.is_satisfied_by(&20));
///
/// let t = combine_specs!(or: spec!(|n: &i32| *n < 0), spec!(|n: &i32| *n > 100));
/// assert!(t.is_satisfied_by(&-1));
/// ```
#[macro_export]
macro_rules! combine_specs {
    (and: $first:expr $(, $rest:expr)* $(,)?) => {{
        use $crate::specification::SpecificationExt;
        let acc = $first;
        $( let acc = acc.and($rest); )*
        acc
    }};
    (or: $first:expr $(, $rest:expr)* $(,)?) => {{
        use $crate::specification::SpecificationExt;
        let acc = $first;
        $( let acc = acc.or($rest); )*
        acc
    }};
    ($first:expr $(, $rest:expr)* $(,)?) => {
        $crate::combine_specs!(and: $first $(, $rest)*)
    };
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closure_spec_works() {
        let s = ClosureSpec::new(|n: &i32| *n > 0);
        assert!(s.is_satisfied_by(&1));
        assert!(!s.is_satisfied_by(&-1));
    }

    #[test]
    fn and_or_not() {
        let gt0 = ClosureSpec::new(|n: &i32| *n > 0);
        let lt10 = ClosureSpec::new(|n: &i32| *n < 10);
        let combined = gt0.and(lt10);
        assert!(combined.is_satisfied_by(&5));
        assert!(!combined.is_satisfied_by(&15));

        let either = ClosureSpec::new(|n: &i32| *n < 0)
            .or(ClosureSpec::new(|n: &i32| *n > 100));
        assert!(either.is_satisfied_by(&-1));
        assert!(!either.is_satisfied_by(&50));

        let neg = ClosureSpec::new(|n: &i32| *n > 0).not();
        assert!(neg.is_satisfied_by(&-1));
    }
}
