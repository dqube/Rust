use uuid::Uuid;

use crate::domain::enums::MovementType;
use crate::domain::ids::{InventoryItemId, StockMovementId};

pub struct CreateInventoryItem {
    pub product_id: Uuid,
    pub product_variant_id: Option<Uuid>,
    pub store_id: i32,
    pub initial_quantity: i32,
    pub reorder_level: i32,
}
ddd_application::impl_command!(CreateInventoryItem, InventoryItemId);

pub struct UpdateInventoryQuantity {
    pub inventory_item_id: InventoryItemId,
    pub new_quantity: i32,
}
ddd_application::impl_command!(UpdateInventoryQuantity, ());

pub struct AdjustInventoryQuantity {
    pub inventory_item_id: InventoryItemId,
    pub delta: i32,
}
ddd_application::impl_command!(AdjustInventoryQuantity, ());

pub struct ReserveStock {
    pub inventory_item_id: InventoryItemId,
    pub quantity: i32,
}
ddd_application::impl_command!(ReserveStock, ());

pub struct ReleaseStock {
    pub inventory_item_id: InventoryItemId,
    pub quantity: i32,
}
ddd_application::impl_command!(ReleaseStock, ());

pub struct RestockInventory {
    pub inventory_item_id: InventoryItemId,
    pub quantity_added: i32,
}
ddd_application::impl_command!(RestockInventory, ());

pub struct UpdateReorderLevel {
    pub inventory_item_id: InventoryItemId,
    pub new_reorder_level: i32,
}
ddd_application::impl_command!(UpdateReorderLevel, ());

pub struct CreateStockMovement {
    pub product_id: Uuid,
    pub product_variant_id: Option<Uuid>,
    pub store_id: i32,
    pub quantity_change: i32,
    pub movement_type: MovementType,
    pub employee_id: Option<Uuid>,
    pub reference_id: Option<String>,
    pub notes: Option<String>,
}
ddd_application::impl_command!(CreateStockMovement, StockMovementId);