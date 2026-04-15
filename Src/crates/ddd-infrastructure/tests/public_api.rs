//! Integration smoke-test: verifies the public API of `ddd-infrastructure`
//! is reachable from an external crate boundary.
//!
//! These tests are deliberately minimal — infrastructure adapters require a
//! live database / broker at test time.  Full integration tests with a real
//! Postgres instance live in the service-level test suites.

/// Confirm the crate compiles and its feature-gated top-level modules are
/// accessible without panicking.
#[test]
fn crate_compiles() {
    // Importing the crate is sufficient to verify the module tree is sound.
    // A future test with `#[cfg(feature = "postgres")]` can create a
    // SeaORM database pool and exercise the repositories.
    let _ = std::any::TypeId::of::<ddd_infrastructure::NatsPublisher>();
}
