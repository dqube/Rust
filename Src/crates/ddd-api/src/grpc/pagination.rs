//! Pagination helpers for gRPC responses.

use ddd_shared_kernel::pagination::{Page, PageRequest};
use serde::{Deserialize, Serialize};

/// Pagination metadata suitable for gRPC responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtoPageInfo {
    /// Current page (one-based).
    pub page: u32,
    /// Items per page.
    pub per_page: u32,
    /// Total items across all pages.
    pub total: u64,
    /// Total number of pages.
    pub total_pages: u32,
    /// Whether a next page exists.
    pub has_next: bool,
    /// Whether a previous page exists.
    pub has_previous: bool,
}

/// Build a [`PageRequest`] from optional gRPC request fields.
///
/// Defaults: page = 1, per_page = 20.
pub fn proto_page_request(page: Option<u32>, per_page: Option<u32>) -> PageRequest {
    PageRequest::new(page.unwrap_or(1), per_page.unwrap_or(20))
}

/// Convert a [`Page<T>`] into a mapped item vec and [`ProtoPageInfo`].
pub fn proto_page_response<T, P>(
    page: Page<T>,
    mapper: impl Fn(T) -> P,
) -> (Vec<P>, ProtoPageInfo) {
    let info = ProtoPageInfo {
        page: page.page(),
        per_page: page.per_page(),
        total: page.total(),
        total_pages: page.total_pages(),
        has_next: page.has_next(),
        has_previous: page.has_previous(),
    };
    let items = page.into_items().into_iter().map(mapper).collect();
    (items, info)
}
