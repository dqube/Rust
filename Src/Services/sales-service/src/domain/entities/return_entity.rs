use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use ddd_domain::{define_aggregate, define_entity, impl_aggregate, impl_aggregate_events};
use ddd_shared_kernel::DomainEvent;

use crate::domain::enums::ReturnReason;
use crate::domain::events::ReturnCreated;
use crate::domain::ids::{ReturnDetailId, ReturnId, SaleId};

// ── ReturnDetail ──────────────────────────────────────────────────────────────

define_entity!(ReturnDetail, ReturnDetailId, {
    pub return_id:  ReturnId,
    pub product_id: Uuid,
    pub quantity:   i32,
    pub reason:     ReturnReason,
    pub restock:    bool,
    pub created_at: DateTime<Utc>,
});

impl ReturnDetail {
    pub fn new(
        return_id:  ReturnId,
        product_id: Uuid,
        quantity:   i32,
        reason:     ReturnReason,
        restock:    bool,
    ) -> Self {
        Self {
            id: ReturnDetailId::new(),
            return_id,
            product_id,
            quantity,
            reason,
            restock,
            created_at: Utc::now(),
        }
    }
}

// ── Return ────────────────────────────────────────────────────────────────────

define_aggregate!(Return, ReturnId, {
    pub sale_id:        SaleId,
    pub return_date:    DateTime<Utc>,
    pub employee_id:    Uuid,
    pub customer_id:    Option<Uuid>,
    pub total_refund:   Decimal,
    pub return_details: Vec<ReturnDetail>,
});

impl_aggregate!(Return, ReturnId);
impl_aggregate_events!(Return);

impl Return {
    pub fn create(sale_id: SaleId, employee_id: Uuid, customer_id: Option<Uuid>) -> Self {
        let now = Utc::now();
        let mut ret = Self {
            id: ReturnId::new(),
            version: 0,
            domain_events: Vec::new(),
            created_at: now,
            updated_at: now,
            sale_id,
            return_date: now,
            employee_id,
            customer_id,
            total_refund: Decimal::ZERO,
            return_details: Vec::new(),
        };
        ret.record_event(ReturnCreated {
            return_id: ret.id,
            sale_id: ret.sale_id,
            occurred_at: now,
        });
        ret
    }

    pub fn add_detail(
        &mut self,
        product_id: Uuid,
        quantity:   i32,
        reason:     ReturnReason,
        restock:    bool,
    ) {
        self.return_details.push(ReturnDetail::new(self.id, product_id, quantity, reason, restock));
        self.updated_at = Utc::now();
    }

    pub fn process(&mut self, total_refund: Decimal) -> Result<(), String> {
        if self.return_details.is_empty() {
            return Err("Cannot process a return with no details.".into());
        }
        self.total_refund = total_refund;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn drain_events(&mut self) -> Vec<Box<dyn DomainEvent>> {
        std::mem::take(&mut self.domain_events)
    }
}
