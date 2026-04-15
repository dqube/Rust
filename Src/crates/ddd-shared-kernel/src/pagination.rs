//! Pagination types.
//!
//! [`PageRequest`] carries the caller's paging intent; [`Page<T>`] is the
//! response wrapper.

use serde::{Deserialize, Serialize};

// ─── PageRequest ─────────────────────────────────────────────────────────────

/// Paging parameters supplied by the caller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageRequest {
    /// One-based page number.
    page: u32,
    /// Number of items per page.
    per_page: u32,
}

impl PageRequest {
    /// Minimum allowed value for `per_page`.
    pub const MIN_PER_PAGE: u32 = 1;
    /// Maximum allowed value for `per_page`.
    pub const MAX_PER_PAGE: u32 = 200;

    /// Create a new `PageRequest`.
    ///
    /// `page` is clamped to `>= 1`; `per_page` is clamped to
    /// `[MIN_PER_PAGE, MAX_PER_PAGE]`.
    pub fn new(page: u32, per_page: u32) -> Self {
        Self {
            page: page.max(1),
            per_page: per_page.clamp(Self::MIN_PER_PAGE, Self::MAX_PER_PAGE),
        }
    }

    /// One-based current page number.
    pub fn page(&self) -> u32 {
        self.page
    }

    /// Items per page.
    pub fn per_page(&self) -> u32 {
        self.per_page
    }

    /// Zero-based row offset for a SQL `OFFSET` clause.
    pub fn offset(&self) -> u64 {
        u64::from(self.page.saturating_sub(1)) * u64::from(self.per_page)
    }
}

impl Default for PageRequest {
    fn default() -> Self {
        Self::new(1, 20)
    }
}

// ─── Page ────────────────────────────────────────────────────────────────────

/// A single page of results with associated metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    /// The items on the current page.
    items: Vec<T>,
    /// Total number of items across all pages.
    total: u64,
    /// One-based current page number.
    page: u32,
    /// Items per page.
    per_page: u32,
    /// Total number of pages.
    total_pages: u32,
}

impl<T> Page<T> {
    /// Construct a `Page` from a slice of items and the total item count.
    ///
    /// `per_page` must be `>= 1`; if `0` is passed it is treated as `1` to
    /// avoid division by zero.
    pub fn new(items: Vec<T>, total: u64, page: u32, per_page: u32) -> Self {
        let per_page = per_page.max(1);
        let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;
        Self {
            items,
            total,
            page,
            per_page,
            total_pages,
        }
    }

    /// Borrow the items.
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// Consume the page and return just the items.
    pub fn into_items(self) -> Vec<T> {
        self.items
    }

    /// Total item count across all pages.
    pub fn total(&self) -> u64 {
        self.total
    }

    /// Current (one-based) page number.
    pub fn page(&self) -> u32 {
        self.page
    }

    /// Items per page.
    pub fn per_page(&self) -> u32 {
        self.per_page
    }

    /// Total number of pages.
    pub fn total_pages(&self) -> u32 {
        self.total_pages
    }

    /// `true` when a next page exists.
    pub fn has_next(&self) -> bool {
        self.page < self.total_pages
    }

    /// `true` when a previous page exists.
    pub fn has_previous(&self) -> bool {
        self.page > 1
    }

    /// Map the items while preserving pagination metadata.
    pub fn map<U, F: FnMut(T) -> U>(self, f: F) -> Page<U> {
        Page {
            items: self.items.into_iter().map(f).collect(),
            total: self.total,
            page: self.page,
            per_page: self.per_page,
            total_pages: self.total_pages,
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_request_offset() {
        let req = PageRequest::new(3, 10);
        assert_eq!(req.offset(), 20);
    }

    #[test]
    fn page_request_clamp() {
        let req = PageRequest::new(0, 0);
        assert_eq!(req.page(), 1);
        assert_eq!(req.per_page(), PageRequest::MIN_PER_PAGE);

        let req = PageRequest::new(1, 9999);
        assert_eq!(req.per_page(), PageRequest::MAX_PER_PAGE);
    }

    #[test]
    fn page_total_pages() {
        let page: Page<u32> = Page::new(vec![1, 2, 3], 23, 1, 10);
        assert_eq!(page.total_pages(), 3);
    }

    #[test]
    fn page_has_next_previous() {
        let page: Page<u32> = Page::new(vec![], 50, 2, 10);
        assert!(page.has_next());
        assert!(page.has_previous());

        let first: Page<u32> = Page::new(vec![], 50, 1, 10);
        assert!(!first.has_previous());

        let last: Page<u32> = Page::new(vec![], 50, 5, 10);
        assert!(!last.has_next());
    }

    #[test]
    fn page_map() {
        let page: Page<u32> = Page::new(vec![1, 2, 3], 3, 1, 10);
        let mapped = page.map(|x| x.to_string());
        assert_eq!(mapped.items(), &["1", "2", "3"]);
    }
}
