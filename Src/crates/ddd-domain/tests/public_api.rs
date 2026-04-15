//! Integration smoke-test: verifies the public API of `ddd-domain`
//! is reachable from an external crate boundary.

use ddd_domain::{Specification, SpecificationExt};

struct AlwaysTrue;
impl<T> Specification<T> for AlwaysTrue {
    fn is_satisfied_by(&self, _candidate: &T) -> bool {
        true
    }
}

struct AlwaysFalse;
impl<T> Specification<T> for AlwaysFalse {
    fn is_satisfied_by(&self, _candidate: &T) -> bool {
        false
    }
}

#[test]
fn specification_and_combinator() {
    let spec = AlwaysTrue.and(AlwaysTrue);
    assert!(spec.is_satisfied_by(&42));
}

#[test]
fn specification_or_combinator() {
    let spec = AlwaysFalse.or(AlwaysTrue);
    assert!(spec.is_satisfied_by(&42));
}

#[test]
fn specification_not_combinator() {
    let spec = AlwaysFalse.not();
    assert!(spec.is_satisfied_by(&42));
}

#[test]
fn specification_and_short_circuits_to_false() {
    let spec = AlwaysFalse.and(AlwaysTrue);
    assert!(!spec.is_satisfied_by(&42));
}
