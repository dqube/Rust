use async_trait::async_trait;
use ddd_domain::Repository;
use ddd_shared_kernel::{AppResult, Page, PageRequest};

use super::aggregate::Product;

#[async_trait]
pub trait ProductRepository: Repository<Product> + Send + Sync {
    async fn find_all_paginated(&self, page: PageRequest) -> AppResult<Page<Product>>;
}
