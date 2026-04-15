//! Pagination helpers shared across API transports.

pub use ddd_shared_kernel::pagination::{Page, PageRequest};

use std::collections::HashMap;

/// Build a [`PageRequest`] from query-string parameters.
///
/// Recognised keys: `page` (one-based, default 1) and `per_page` (default 20).
pub fn page_request_from_query(params: &HashMap<String, String>) -> PageRequest {
    let page = params
        .get("page")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(1);
    let per_page = params
        .get("per_page")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(20);
    PageRequest::new(page, per_page)
}
