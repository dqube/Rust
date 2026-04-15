//! Pagination re-exports and helpers.

pub use ddd_shared_kernel::pagination::{Page, PageRequest};

/// Build a [`PageRequest`] from optional query-string params, applying
/// defaults when either is missing.
pub fn page_request_from_params(page: Option<u32>, per_page: Option<u32>) -> PageRequest {
    PageRequest::new(page.unwrap_or(1), per_page.unwrap_or(20))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_applied() {
        let r = page_request_from_params(None, None);
        assert_eq!(r.page(), 1);
        assert_eq!(r.per_page(), 20);
    }

    #[test]
    fn explicit_values_used() {
        let r = page_request_from_params(Some(3), Some(50));
        assert_eq!(r.page(), 3);
        assert_eq!(r.per_page(), 50);
    }
}
