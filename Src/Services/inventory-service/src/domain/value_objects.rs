use chrono::Utc;
use ddd_shared_kernel::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryLocator {
    pub product_id: Uuid,
    pub product_variant_id: Option<Uuid>,
    pub store_id: i32,
}

impl InventoryLocator {
    pub fn new(
        product_id: Uuid,
        product_variant_id: Option<Uuid>,
        store_id: i32,
    ) -> AppResult<Self> {
        if product_id.is_nil() {
            return Err(AppError::validation("product_id", "must not be nil"));
        }
        if matches!(product_variant_id, Some(variant) if variant.is_nil()) {
            return Err(AppError::validation("product_variant_id", "must not be nil"));
        }
        if store_id <= 0 {
            return Err(AppError::validation("store_id", "must be positive"));
        }
        Ok(Self {
            product_id,
            product_variant_id,
            store_id,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockLevel {
    pub quantity: i32,
    pub reserved_quantity: i32,
    pub measured_at: chrono::DateTime<Utc>,
}

impl StockLevel {
    pub fn new(quantity: i32, reserved_quantity: i32) -> AppResult<Self> {
        if quantity < 0 {
            return Err(AppError::validation("quantity", "must be non-negative"));
        }
        if reserved_quantity < 0 {
            return Err(AppError::validation(
                "reserved_quantity",
                "must be non-negative",
            ));
        }
        if reserved_quantity > quantity {
            return Err(AppError::validation(
                "reserved_quantity",
                "must not exceed quantity",
            ));
        }
        Ok(Self {
            quantity,
            reserved_quantity,
            measured_at: Utc::now(),
        })
    }

    pub fn available_quantity(&self) -> i32 {
        self.quantity - self.reserved_quantity
    }
}