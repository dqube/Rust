use chrono::Utc;

use ddd_domain::{define_aggregate, impl_aggregate, impl_aggregate_events};
use ddd_shared_kernel::{AppError, AppResult};
use uuid::Uuid;

use crate::domain::enums::MovementType;
use crate::domain::events::StockMovementCreated;
use crate::domain::ids::StockMovementId;
use crate::domain::value_objects::InventoryLocator;

define_aggregate!(StockMovement, StockMovementId, {
    pub locator: InventoryLocator,
    pub quantity_change: i32,
    pub movement_type: MovementType,
    pub movement_date: chrono::DateTime<Utc>,
    pub employee_id: Option<Uuid>,
    pub reference_id: Option<String>,
    pub notes: Option<String>,
});

impl_aggregate!(StockMovement, StockMovementId);
impl_aggregate_events!(StockMovement);

impl StockMovement {
    pub fn create(
        id: StockMovementId,
        locator: InventoryLocator,
        quantity_change: i32,
        movement_type: MovementType,
        employee_id: Option<Uuid>,
        reference_id: Option<String>,
        notes: Option<String>,
    ) -> AppResult<Self> {
        if quantity_change == 0 {
            return Err(AppError::validation(
                "quantity_change",
                "must not be zero",
            ));
        }

        let now = Utc::now();
        let mut movement = Self {
            id,
            version: 0,
            domain_events: Vec::new(),
            created_at: now,
            updated_at: now,
            locator,
            quantity_change,
            movement_type,
            movement_date: now,
            employee_id,
            reference_id,
            notes,
        };
        movement.domain_events.push(Box::new(StockMovementCreated {
            stock_movement_id: id,
            occurred_at: now,
        }));
        Ok(movement)
    }
}