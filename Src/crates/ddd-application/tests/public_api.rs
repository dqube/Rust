//! Integration smoke-test: verifies the public API of `ddd-application`
//! is reachable from an external crate boundary.

use ddd_application::pagination::{Page, PageRequest};

#[test]
fn page_request_explicit() {
    let pr = PageRequest::new(2, 15);
    assert_eq!(pr.page(), 2);
    assert_eq!(pr.per_page(), 15);
    assert_eq!(pr.offset(), 15);
}

#[test]
fn page_has_next_previous() {
    let page: Page<i32> = Page::new(vec![6, 7, 8, 9, 10], 15, 2, 5);
    assert!(page.has_previous(), "page 2 should have a previous page");
    assert!(page.has_next(), "5 items with 15 total on page 2 should have next");
}

#[test]
fn page_last_page_no_next() {
    let page: Page<i32> = Page::new(vec![11, 12, 13, 14, 15], 15, 3, 5);
    assert!(!page.has_next(), "page 3 of 3 should not have a next page");
}
