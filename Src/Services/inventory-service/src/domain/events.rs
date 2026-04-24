use std::any::Any;

use chrono::{DateTime, Utc};
use ddd_shared_kernel::DomainEvent;
use serde::{Deserialize, Serialize};

use super::ids::{InventoryItemId, StockMovementId};

macro_rules! domain_event {
    ($ty:ident, $name:literal) => {
        impl DomainEvent for $ty {
            fn event_name(&self) -> &'static str {
                $name
            }

            fn occurred_at(&self) -> DateTime<Utc> {
                self.occurred_at
            }

            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItemCreated {
    pub inventory_item_id: InventoryItemId,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(InventoryItemCreated, "inventory.item.created");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryQuantityChanged {
    pub inventory_item_id: InventoryItemId,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(InventoryQuantityChanged, "inventory.item.quantity_changed");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMovementCreated {
    pub stock_movement_id: StockMovementId,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(StockMovementCreated, "inventory.stock_movement.created");