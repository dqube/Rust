use ddd_shared_kernel::Page;
use uuid::Uuid;

use crate::domain::entities::{InventoryItem, StockMovement};
use crate::domain::enums::MovementType;
use crate::domain::ids::InventoryItemId;

pub struct GetInventoryItem {
    pub inventory_item_id: InventoryItemId,
}
ddd_application::impl_query!(GetInventoryItem, Option<InventoryItem>);

pub struct ListInventoryItems {
    pub page: u32,
    pub per_page: u32,
}
ddd_application::impl_query!(ListInventoryItems, Page<InventoryItem>);

pub struct GetInventoryByStore {
    pub store_id: i32,
    pub page: u32,
    pub per_page: u32,
}
ddd_application::impl_query!(GetInventoryByStore, Page<InventoryItem>);

pub struct GetInventoryByProduct {
    pub product_id: Uuid,
}
ddd_application::impl_query!(GetInventoryByProduct, Vec<InventoryItem>);

pub struct GetLowStockItems {
    pub store_id: Option<i32>,
    pub page: u32,
    pub per_page: u32,
}
ddd_application::impl_query!(GetLowStockItems, Page<InventoryItem>);

pub struct GetOutOfStockItems {
    pub store_id: Option<i32>,
    pub page: u32,
    pub per_page: u32,
}
ddd_application::impl_query!(GetOutOfStockItems, Page<InventoryItem>);

pub struct GetStockMovements {
    pub product_id: Option<Uuid>,
    pub store_id: Option<i32>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub movement_type: Option<MovementType>,
    pub page: u32,
    pub per_page: u32,
}
ddd_application::impl_query!(GetStockMovements, Page<StockMovement>);