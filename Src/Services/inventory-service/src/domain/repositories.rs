use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ddd_shared_kernel::{AppResult, Page, PageRequest};
use uuid::Uuid;

use super::entities::{InventoryItem, StockMovement};
use super::enums::MovementType;
use super::ids::{InventoryItemId, StockMovementId};
use super::value_objects::InventoryLocator;

#[derive(Debug, Clone, Default)]
pub struct StockMovementFilter {
    pub product_id: Option<Uuid>,
    pub store_id: Option<i32>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub movement_type: Option<MovementType>,
}

#[async_trait]
pub trait InventoryItemRepository: Send + Sync {
    async fn find_by_id(&self, id: InventoryItemId) -> AppResult<Option<InventoryItem>>;
    async fn save(&self, item: &InventoryItem) -> AppResult<()>;
    async fn exists(&self, locator: InventoryLocator) -> AppResult<bool>;
    async fn has_variant_inventory(&self, locator: &InventoryLocator) -> AppResult<bool>;
    async fn has_product_level_inventory(&self, locator: &InventoryLocator) -> AppResult<bool>;
    async fn list_paged(&self, page: PageRequest) -> AppResult<Page<InventoryItem>>;
    async fn list_by_store(&self, store_id: i32, page: PageRequest) -> AppResult<Page<InventoryItem>>;
    async fn list_by_product(&self, product_id: Uuid) -> AppResult<Vec<InventoryItem>>;
    async fn list_low_stock(&self, store_id: Option<i32>, page: PageRequest) -> AppResult<Page<InventoryItem>>;
    async fn list_out_of_stock(&self, store_id: Option<i32>, page: PageRequest) -> AppResult<Page<InventoryItem>>;
}

#[async_trait]
pub trait StockMovementRepository: Send + Sync {
    async fn find_by_id(&self, id: StockMovementId) -> AppResult<Option<StockMovement>>;
    async fn save(&self, movement: &StockMovement) -> AppResult<()>;
    async fn list(
        &self,
        filter: StockMovementFilter,
        page: PageRequest,
    ) -> AppResult<Page<StockMovement>>;
}