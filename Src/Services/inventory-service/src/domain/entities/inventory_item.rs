use chrono::Utc;

use ddd_domain::{define_aggregate, impl_aggregate, impl_aggregate_events};
use ddd_shared_kernel::{AppError, AppResult};

use crate::domain::events::{InventoryItemCreated, InventoryQuantityChanged};
use crate::domain::ids::InventoryItemId;
use crate::domain::value_objects::{InventoryLocator, StockLevel};

define_aggregate!(InventoryItem, InventoryItemId, {
    pub locator: InventoryLocator,
    pub quantity: i32,
    pub reserved_quantity: i32,
    pub reorder_level: i32,
    pub last_restock_date: Option<chrono::DateTime<Utc>>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
});

impl_aggregate!(InventoryItem, InventoryItemId);
impl_aggregate_events!(InventoryItem);

impl InventoryItem {
    pub fn create(
        id: InventoryItemId,
        locator: InventoryLocator,
        initial_quantity: i32,
        reorder_level: i32,
    ) -> AppResult<Self> {
        if initial_quantity < 0 {
            return Err(AppError::validation(
                "initial_quantity",
                "must be non-negative",
            ));
        }
        if reorder_level < 0 {
            return Err(AppError::validation(
                "reorder_level",
                "must be non-negative",
            ));
        }

        let now = Utc::now();
        let mut item = Self {
            id,
            version: 0,
            domain_events: Vec::new(),
            created_at: now,
            updated_at: now,
            locator,
            quantity: initial_quantity,
            reserved_quantity: 0,
            reorder_level,
            last_restock_date: None,
            created_by: None,
            updated_by: None,
        };
        item.ensure_consistent()?;
        item.domain_events.push(Box::new(InventoryItemCreated {
            inventory_item_id: id,
            occurred_at: now,
        }));
        Ok(item)
    }

    pub fn available_quantity(&self) -> i32 {
        self.quantity - self.reserved_quantity
    }

    pub fn is_low_stock(&self) -> bool {
        self.quantity > 0 && self.quantity <= self.reorder_level
    }

    pub fn is_out_of_stock(&self) -> bool {
        self.quantity == 0
    }

    pub fn update_quantity(&mut self, new_quantity: i32) -> AppResult<()> {
        self.quantity = new_quantity;
        self.ensure_consistent()?;
        self.mark_quantity_changed();
        Ok(())
    }

    pub fn adjust_quantity(&mut self, delta: i32) -> AppResult<()> {
        self.quantity += delta;
        self.ensure_consistent()?;
        self.mark_quantity_changed();
        Ok(())
    }

    pub fn reserve_stock(&mut self, quantity: i32) -> AppResult<()> {
        if quantity <= 0 {
            return Err(AppError::validation("quantity", "must be positive"));
        }
        if quantity > self.available_quantity() {
            return Err(AppError::conflict("insufficient available stock"));
        }
        self.reserved_quantity += quantity;
        self.ensure_consistent()?;
        self.mark_quantity_changed();
        Ok(())
    }

    pub fn release_stock(&mut self, quantity: i32) -> AppResult<()> {
        if quantity <= 0 {
            return Err(AppError::validation("quantity", "must be positive"));
        }
        if quantity > self.reserved_quantity {
            return Err(AppError::validation(
                "quantity",
                "must not exceed reserved stock",
            ));
        }
        self.reserved_quantity -= quantity;
        self.ensure_consistent()?;
        self.mark_quantity_changed();
        Ok(())
    }

    pub fn record_restock(&mut self, quantity_added: i32) -> AppResult<()> {
        if quantity_added <= 0 {
            return Err(AppError::validation(
                "quantity_added",
                "must be positive",
            ));
        }
        self.quantity += quantity_added;
        self.last_restock_date = Some(Utc::now());
        self.ensure_consistent()?;
        self.mark_quantity_changed();
        Ok(())
    }

    pub fn update_reorder_level(&mut self, new_reorder_level: i32) -> AppResult<()> {
        if new_reorder_level < 0 {
            return Err(AppError::validation(
                "new_reorder_level",
                "must be non-negative",
            ));
        }
        self.reorder_level = new_reorder_level;
        self.updated_at = Utc::now();
        Ok(())
    }

    fn ensure_consistent(&self) -> AppResult<()> {
        let _ = StockLevel::new(self.quantity, self.reserved_quantity)?;
        Ok(())
    }

    fn mark_quantity_changed(&mut self) {
        let now = Utc::now();
        self.updated_at = now;
        self.domain_events.push(Box::new(InventoryQuantityChanged {
            inventory_item_id: self.id,
            occurred_at: now,
        }));
    }
}