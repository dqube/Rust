//! Pagination DTOs and API response wrappers for REST APIs.

use ddd_shared_kernel::pagination::Page;
use serde::{Deserialize, Serialize};

use super::problem_details::ProblemDetail;

/// Paginated response DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PageDto<T> {
    /// Items on the current page.
    pub items: Vec<T>,
    /// Current page number (one-based).
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

impl<T> From<Page<T>> for PageDto<T> {
    fn from(p: Page<T>) -> Self {
        Self {
            page: p.page(),
            per_page: p.per_page(),
            total: p.total(),
            total_pages: p.total_pages(),
            has_next: p.has_next(),
            has_previous: p.has_previous(),
            items: p.into_items(),
        }
    }
}

/// Standard API response envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ApiResponse<T> {
    /// Whether the request succeeded.
    pub success: bool,
    /// The response payload (present on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// The error detail (present on failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ProblemDetail>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Build a success response.
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Build an error response.
    pub fn err(detail: ProblemDetail) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(detail),
        }
    }
}
