use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::enums::OrderSagaStep;
use crate::domain::ids::SaleId;

// ── SagaOrderItem (value object) ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaOrderItem {
    pub product_id: Uuid,
    pub quantity:   i32,
    pub unit_price: Decimal,
}

// ── OrderSaga ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct OrderSaga {
    pub order_id:       SaleId,
    pub order_number:   String,
    pub customer_id:    Uuid,
    pub store_id:       i32,
    pub total:          Decimal,
    pub reservation_id: Option<Uuid>,
    pub payment_id:     Option<Uuid>,
    pub step:           OrderSagaStep,
    pub failure_reason: Option<String>,
    pub items:          Vec<SagaOrderItem>,
    pub created_at:     DateTime<Utc>,
    pub updated_at:     DateTime<Utc>,
}
