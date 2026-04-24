use chrono::{DateTime, Utc};
use ddd_shared_kernel::DomainEvent;
use serde::{Deserialize, Serialize};
use std::any::Any;

use crate::domain::ids::{ReturnId, SaleId};

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

// ── Sale lifecycle ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleCreated {
    pub sale_id:        SaleId,
    pub store_id:       i32,
    pub receipt_number: String,
    pub occurred_at:    DateTime<Utc>,
}
domain_event!(SaleCreated, "sales.sale.created");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleCompleted {
    pub sale_id:     SaleId,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(SaleCompleted, "sales.sale.completed");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleCancelled {
    pub sale_id:     SaleId,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(SaleCancelled, "sales.sale.cancelled");

// ── Return lifecycle ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnCreated {
    pub return_id:   ReturnId,
    pub sale_id:     SaleId,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(ReturnCreated, "sales.return.created");
