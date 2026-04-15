//! Integration smoke-test: verifies the public API of `ddd-shared-kernel`
//! is reachable from an external crate boundary.

use ddd_shared_kernel::{
    AppError, AppResult,
    pagination::{Page, PageRequest},
};

#[test]
fn app_error_not_found_round_trips() {
    let err = AppError::not_found("Order", "123");
    let msg = err.to_string();
    assert!(msg.contains("Order") && msg.contains("123"), "unexpected: {msg}");
}

#[test]
fn page_request_defaults() {
    let pr = PageRequest::default();
    assert_eq!(pr.page(), 1);
    assert!(pr.per_page() > 0);
}

#[test]
fn page_maps_items() {
    let page: Page<i32> = Page::new(vec![1, 2, 3], 3, 1, 10);
    assert_eq!(page.items().len(), 3);
    assert_eq!(page.total(), 3);
}

#[test]
fn app_result_ok_propagates() {
    fn divide(a: i32, b: i32) -> AppResult<i32> {
        if b == 0 {
            return Err(AppError::internal("division by zero"));
        }
        Ok(a / b)
    }
    assert_eq!(divide(10, 2).unwrap(), 5);
    assert!(divide(10, 0).is_err());
}
