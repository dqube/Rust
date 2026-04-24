use std::sync::Arc;

use crate::domain::repositories::{InventoryItemRepository, StockMovementRepository};

#[derive(Clone)]
pub struct AppDeps {
    pub inventory_repo: Arc<dyn InventoryItemRepository>,
    pub stock_movement_repo: Arc<dyn StockMovementRepository>,
}