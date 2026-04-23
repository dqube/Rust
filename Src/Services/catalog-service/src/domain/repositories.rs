use async_trait::async_trait;
use ddd_shared_kernel::{AppError, Page, PageRequest};

use super::entities::{Brand, Product, ProductCategory, TaxConfiguration};
use super::ids::{BrandId, CategoryId, ProductId, TaxConfigId};

#[async_trait]
pub trait ProductRepository: Send + Sync {
    async fn find_by_id(&self, id: ProductId) -> Result<Option<Product>, AppError>;
    async fn find_by_sku(&self, sku: &str) -> Result<Option<Product>, AppError>;
    async fn get_paged(
        &self,
        search:      Option<&str>,
        category_id: Option<i32>,
        min_price:   Option<f64>,
        max_price:   Option<f64>,
        sort_by:     Option<&str>,
        sort_desc:   bool,
        req:         &PageRequest,
    ) -> Result<Page<Product>, AppError>;
    async fn save(&self, product: &Product) -> Result<(), AppError>;
}

#[async_trait]
pub trait CategoryRepository: Send + Sync {
    async fn find_by_id(&self, id: CategoryId) -> Result<Option<ProductCategory>, AppError>;
    async fn get_all(&self, parent_id: Option<i32>) -> Result<Vec<ProductCategory>, AppError>;
    /// Inserts and returns the DB-assigned SERIAL id.
    async fn insert(&self, category: &ProductCategory) -> Result<i32, AppError>;
    async fn save(&self, category: &ProductCategory) -> Result<(), AppError>;
    async fn delete(&self, id: CategoryId) -> Result<(), AppError>;
}

#[async_trait]
pub trait BrandRepository: Send + Sync {
    async fn find_by_id(&self, id: BrandId) -> Result<Option<Brand>, AppError>;
    async fn get_paged(
        &self,
        search:      Option<&str>,
        active_only: bool,
        req:         &PageRequest,
    ) -> Result<Page<Brand>, AppError>;
    async fn save(&self, brand: &Brand) -> Result<(), AppError>;
}

#[async_trait]
pub trait TaxConfigRepository: Send + Sync {
    async fn find_by_id(&self, id: TaxConfigId) -> Result<Option<TaxConfiguration>, AppError>;
    async fn get_filtered(
        &self,
        location_id: Option<i32>,
        tax_type:    Option<&str>,
        active_only: bool,
    ) -> Result<Vec<TaxConfiguration>, AppError>;
    async fn get_applicable(
        &self,
        location_id: i32,
        category_id: Option<i32>,
    ) -> Result<Vec<TaxConfiguration>, AppError>;
    async fn save(&self, tc: &TaxConfiguration) -> Result<(), AppError>;
    async fn delete(&self, id: TaxConfigId) -> Result<(), AppError>;
}
