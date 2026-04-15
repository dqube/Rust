//! Integration smoke-test: verifies the public API of `ddd-bff`
//! is reachable from an external crate boundary.

use ddd_bff::{grpc_status_to_app_error, app_error_to_problem};
use ddd_shared_kernel::AppError;

#[test]
fn not_found_status_maps_to_app_error() {
    let status = tonic::Status::not_found("order 99 not found");
    let err = grpc_status_to_app_error(status);
    assert!(matches!(err, AppError::NotFound { .. }), "expected NotFound, got {err:?}");
}

#[test]
fn permission_denied_maps_to_forbidden() {
    let status = tonic::Status::permission_denied("access denied");
    let err = grpc_status_to_app_error(status);
    assert!(matches!(err, AppError::Forbidden { .. }), "expected Forbidden, got {err:?}");
}

#[test]
fn internal_error_maps_to_problem_detail_500() {
    let err = AppError::internal("something broke");
    let pd = app_error_to_problem(&err);
    assert_eq!(pd.status, 500);
}

#[test]
fn not_found_error_maps_to_problem_detail_404() {
    let err = AppError::not_found("Order", "42");
    let pd = app_error_to_problem(&err);
    assert_eq!(pd.status, 404);
}
