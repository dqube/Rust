use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use ddd_domain::{define_aggregate, define_entity, impl_aggregate, impl_aggregate_events};
use ddd_shared_kernel::DomainEvent;

use crate::domain::enums::{OrderStatus, SalesChannel};
use crate::domain::events::{SaleCancelled, SaleCompleted, SaleCreated};
use crate::domain::ids::{AppliedDiscountId, SaleDetailId, SaleId};

// ── Address (value object) ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub street:   String,
    pub city:     String,
    pub state:    String,
    pub zip_code: String,
    pub country:  String,
}

// ── SaleDetail ────────────────────────────────────────────────────────────────

define_entity!(SaleDetail, SaleDetailId, {
    pub sale_id:          SaleId,
    pub product_id:       Uuid,
    pub variant_id:       Option<Uuid>,
    pub quantity:         i32,
    pub unit_price:       Decimal,
    pub applied_discount: Decimal,
    pub tax_applied:      Decimal,
    pub line_total:       Decimal,
    pub created_at:       DateTime<Utc>,
});

impl SaleDetail {
    pub fn new(
        sale_id:     SaleId,
        product_id:  Uuid,
        variant_id:  Option<Uuid>,
        quantity:    i32,
        unit_price:  Decimal,
        tax_applied: Decimal,
    ) -> Self {
        let line_total = unit_price * Decimal::from(quantity);
        Self {
            id: SaleDetailId::new(),
            sale_id,
            product_id,
            variant_id,
            quantity,
            unit_price,
            applied_discount: Decimal::ZERO,
            tax_applied,
            line_total,
            created_at: Utc::now(),
        }
    }

    pub fn update(&mut self, quantity: i32, unit_price: Decimal, tax_applied: Decimal) {
        self.quantity    = quantity;
        self.unit_price  = unit_price;
        self.tax_applied = tax_applied;
        self.recalc();
    }

    pub fn recalc(&mut self) {
        self.line_total = (self.unit_price * Decimal::from(self.quantity)) - self.applied_discount;
    }
}

// ── AppliedDiscount ───────────────────────────────────────────────────────────

define_entity!(AppliedDiscount, AppliedDiscountId, {
    pub sale_id:         SaleId,
    pub sale_detail_id:  Option<SaleDetailId>,
    pub campaign_id:     Uuid,
    pub rule_id:         Uuid,
    pub discount_amount: Decimal,
    pub created_at:      DateTime<Utc>,
});

impl AppliedDiscount {
    pub fn new(
        sale_id:         SaleId,
        sale_detail_id:  Option<SaleDetailId>,
        campaign_id:     Uuid,
        rule_id:         Uuid,
        discount_amount: Decimal,
    ) -> Self {
        Self {
            id: AppliedDiscountId::new(),
            sale_id,
            sale_detail_id,
            campaign_id,
            rule_id,
            discount_amount,
            created_at: Utc::now(),
        }
    }
}

// ── Sale ──────────────────────────────────────────────────────────────────────

define_aggregate!(Sale, SaleId, {
    pub store_id:               i32,
    pub employee_id:            Uuid,
    pub customer_id:            Option<Uuid>,
    pub register_id:            i32,
    pub receipt_number:         String,
    pub transaction_time:       DateTime<Utc>,
    pub sub_total:              Decimal,
    pub discount_total:         Decimal,
    pub tax_amount:             Decimal,
    pub total_amount:           Decimal,
    pub channel:                SalesChannel,
    pub status:                 OrderStatus,
    pub shipping_address:       Option<Address>,
    pub billing_address:        Option<Address>,
    pub payment_transaction_id: Option<String>,
    pub receipt_object_name:    Option<String>,
    pub sale_details:           Vec<SaleDetail>,
    pub applied_discounts:      Vec<AppliedDiscount>,
});

impl_aggregate!(Sale, SaleId);
impl_aggregate_events!(Sale);

impl Sale {
    pub fn create(
        store_id:       i32,
        employee_id:    Uuid,
        register_id:    i32,
        receipt_number: String,
        customer_id:    Option<Uuid>,
        channel:        SalesChannel,
    ) -> Self {
        let now = Utc::now();
        let mut s = Self {
            id: SaleId::new(),
            version: 0,
            domain_events: Vec::new(),
            created_at: now,
            updated_at: now,
            store_id,
            employee_id,
            customer_id,
            register_id,
            receipt_number,
            transaction_time: now,
            sub_total: Decimal::ZERO,
            discount_total: Decimal::ZERO,
            tax_amount: Decimal::ZERO,
            total_amount: Decimal::ZERO,
            channel,
            status: OrderStatus::Pending,
            shipping_address: None,
            billing_address: None,
            payment_transaction_id: None,
            receipt_object_name: None,
            sale_details: Vec::new(),
            applied_discounts: Vec::new(),
        };
        s.record_event(SaleCreated {
            sale_id:        s.id,
            store_id:       s.store_id,
            receipt_number: s.receipt_number.clone(),
            occurred_at:    now,
        });
        s
    }

    pub fn place_online_order(
        customer_id: Uuid,
        store_id:    i32,
        items:       Vec<(Uuid, i32, Decimal)>,
    ) -> Self {
        let receipt = format!("ORD-{}", &uuid::Uuid::new_v4().to_string()[..8].to_uppercase());
        let mut sale = Sale::create(store_id, Uuid::nil(), 0, receipt, Some(customer_id), SalesChannel::Online);
        sale.status = OrderStatus::Pending;
        for (product_id, qty, price) in items {
            let det = SaleDetail::new(sale.id, product_id, None, qty, price, Decimal::ZERO);
            sale.sale_details.push(det);
        }
        sale.recalculate_totals();
        sale
    }

    pub fn add_detail(
        &mut self,
        product_id:  Uuid,
        variant_id:  Option<Uuid>,
        quantity:    i32,
        unit_price:  Decimal,
        tax_applied: Decimal,
    ) -> SaleDetailId {
        if let Some(det) = self.sale_details.iter_mut().find(|d| d.product_id == product_id && d.variant_id == variant_id) {
            det.quantity    += quantity;
            det.tax_applied += tax_applied;
            det.recalc();
            let id = det.id;
            self.recalculate_totals();
            return id;
        }
        let det = SaleDetail::new(self.id, product_id, variant_id, quantity, unit_price, tax_applied);
        let id = det.id;
        self.sale_details.push(det);
        self.recalculate_totals();
        id
    }

    pub fn update_detail(
        &mut self,
        detail_id:   SaleDetailId,
        quantity:    i32,
        unit_price:  Decimal,
        tax_applied: Decimal,
    ) -> Result<(), String> {
        let det = self.sale_details.iter_mut().find(|d| d.id == detail_id)
            .ok_or_else(|| format!("SaleDetail {} not found.", detail_id))?;
        det.update(quantity, unit_price, tax_applied);
        self.recalculate_totals();
        Ok(())
    }

    pub fn remove_detail(&mut self, detail_id: SaleDetailId) -> Result<(), String> {
        let pos = self.sale_details.iter().position(|d| d.id == detail_id)
            .ok_or_else(|| format!("SaleDetail {} not found.", detail_id))?;
        self.sale_details.remove(pos);
        self.applied_discounts.retain(|d| d.sale_detail_id != Some(detail_id));
        self.recalculate_totals();
        Ok(())
    }

    pub fn apply_discount(
        &mut self,
        sale_detail_id: Option<SaleDetailId>,
        campaign_id:    Uuid,
        rule_id:        Uuid,
        amount:         Decimal,
    ) {
        if let Some(det_id) = sale_detail_id {
            if let Some(det) = self.sale_details.iter_mut().find(|d| d.id == det_id) {
                det.applied_discount += amount;
                det.recalc();
            }
        }
        let disc = AppliedDiscount::new(self.id, sale_detail_id, campaign_id, rule_id, amount);
        self.applied_discounts.push(disc);
        self.recalculate_totals();
    }

    pub fn complete(&mut self) {
        let now = Utc::now();
        self.status = OrderStatus::Completed;
        self.updated_at = now;
        self.record_event(SaleCompleted { sale_id: self.id, occurred_at: now });
    }

    pub fn set_addresses(&mut self, shipping: Address, billing: Address) {
        self.shipping_address = Some(shipping);
        self.billing_address  = Some(billing);
        self.updated_at = Utc::now();
    }

    pub fn set_payment_transaction(&mut self, tx_id: String) {
        self.payment_transaction_id = Some(tx_id);
        self.status = OrderStatus::Paid;
        self.updated_at = Utc::now();
    }

    pub fn mark_shipped(&mut self) -> Result<(), String> {
        match self.status {
            OrderStatus::Paid | OrderStatus::Processing => {
                self.status = OrderStatus::Shipped;
                self.updated_at = Utc::now();
                Ok(())
            }
            _ => Err(format!("Cannot mark shipped from status {:?}", self.status)),
        }
    }

    pub fn mark_delivered(&mut self) -> Result<(), String> {
        if self.status == OrderStatus::Shipped {
            self.status = OrderStatus::Delivered;
            self.updated_at = Utc::now();
            Ok(())
        } else {
            Err("Can only deliver a shipped order.".into())
        }
    }

    pub fn cancel(&mut self, _reason: &str) -> Result<(), String> {
        match self.status {
            OrderStatus::Shipped | OrderStatus::Delivered | OrderStatus::Completed =>
                Err("Cannot cancel a shipped/delivered/completed order.".into()),
            _ => {
                let now = Utc::now();
                self.status = OrderStatus::Cancelled;
                self.updated_at = now;
                self.record_event(SaleCancelled { sale_id: self.id, occurred_at: now });
                Ok(())
            }
        }
    }

    pub fn anonymize_customer(&mut self) {
        self.customer_id = None;
        self.updated_at = Utc::now();
    }

    pub fn set_receipt_object_name(&mut self, name: String) {
        self.receipt_object_name = Some(name);
        self.updated_at = Utc::now();
    }

    pub fn drain_events(&mut self) -> Vec<Box<dyn DomainEvent>> {
        std::mem::take(&mut self.domain_events)
    }

    fn recalculate_totals(&mut self) {
        self.sub_total      = self.sale_details.iter().map(|d| d.unit_price * Decimal::from(d.quantity)).sum();
        self.discount_total = self.applied_discounts.iter().map(|d| d.discount_amount).sum();
        self.tax_amount     = self.sale_details.iter().map(|d| d.tax_applied).sum();
        self.total_amount   = self.sub_total + self.tax_amount - self.discount_total;
    }
}
