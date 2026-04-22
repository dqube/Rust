//! Order repository port.

use async_trait::async_trait;
use ddd_domain::Repository;
use ddd_shared_kernel::{AppResult, Page, PageRequest};

use super::aggregate::Order;

/// Repository for the Order aggregate.
#[async_trait]
pub trait OrderRepository: Repository<Order> + Send + Sync {
    /// List orders with pagination.
    async fn find_all_paginated(&self, page: PageRequest) -> AppResult<Page<Order>>;

    /// Find orders by customer ID.
    async fn find_by_customer(&self, customer_id: &str) -> AppResult<Vec<Order>>;
}
