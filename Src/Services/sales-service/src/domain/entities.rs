use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{
    enums::{OrderSagaStep, OrderStatus, ReturnReason, SalesChannel},
    ids::*,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub street:   String,
    pub city:     String,
    pub state:    String,
    pub zip_code: String,
    pub country:  String,
}

#[derive(Debug, Clone)]
pub struct SaleDetail {
    pub id:               SaleDetailId,
    pub sale_id:          SaleId,
    pub product_id:       Uuid,
    pub variant_id:       Option<Uuid>,
    pub quantity:         i32,
    pub unit_price:       Decimal,
    pub applied_discount: Decimal,
    pub tax_applied:      Decimal,
    pub line_total:       Decimal,
    pub created_at:       DateTime<Utc>,
}

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

#[derive(Debug, Clone)]
pub struct AppliedDiscount {
    pub id:              AppliedDiscountId,
    pub sale_id:         SaleId,
    pub sale_detail_id:  Option<SaleDetailId>,
    pub campaign_id:     Uuid,
    pub rule_id:         Uuid,
    pub discount_amount: Decimal,
    pub created_at:      DateTime<Utc>,
}

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

#[derive(Debug, Clone)]
pub struct Sale {
    pub id:                     SaleId,
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
    pub created_at:             DateTime<Utc>,
    // Loaded separately
    pub sale_details:           Vec<SaleDetail>,
    pub applied_discounts:      Vec<AppliedDiscount>,
}

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
        Self {
            id: SaleId::new(),
            store_id,
            employee_id,
            customer_id,
            register_id,
            receipt_number,
            transaction_time:       now,
            sub_total:              Decimal::ZERO,
            discount_total:         Decimal::ZERO,
            tax_amount:             Decimal::ZERO,
            total_amount:           Decimal::ZERO,
            channel,
            status:                 OrderStatus::Pending,
            shipping_address:       None,
            billing_address:        None,
            payment_transaction_id: None,
            receipt_object_name:    None,
            created_at:             now,
            sale_details:           Vec::new(),
            applied_discounts:      Vec::new(),
        }
    }

    pub fn place_online_order(
        customer_id: Uuid,
        store_id:    i32,
        items:       Vec<(Uuid, i32, Decimal)>, // (product_id, qty, unit_price)
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
        product_id: Uuid,
        variant_id: Option<Uuid>,
        quantity:   i32,
        unit_price: Decimal,
        tax_applied: Decimal,
    ) -> SaleDetailId {
        if let Some(det) = self.sale_details.iter_mut().find(|d| d.product_id == product_id && d.variant_id == variant_id) {
            det.quantity   += quantity;
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
        self.status = OrderStatus::Completed;
    }

    pub fn set_addresses(&mut self, shipping: Address, billing: Address) {
        self.shipping_address = Some(shipping);
        self.billing_address  = Some(billing);
    }

    pub fn set_payment_transaction(&mut self, tx_id: String) {
        self.payment_transaction_id = Some(tx_id);
        self.status = OrderStatus::Paid;
    }

    pub fn mark_shipped(&mut self) -> Result<(), String> {
        match self.status {
            OrderStatus::Paid | OrderStatus::Processing => {
                self.status = OrderStatus::Shipped;
                Ok(())
            }
            _ => Err(format!("Cannot mark shipped from status {:?}", self.status)),
        }
    }

    pub fn mark_delivered(&mut self) -> Result<(), String> {
        if self.status == OrderStatus::Shipped {
            self.status = OrderStatus::Delivered;
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
                self.status = OrderStatus::Cancelled;
                Ok(())
            }
        }
    }

    pub fn anonymize_customer(&mut self) {
        self.customer_id = None;
    }

    pub fn set_receipt_object_name(&mut self, name: String) {
        self.receipt_object_name = Some(name);
    }

    fn recalculate_totals(&mut self) {
        self.sub_total      = self.sale_details.iter().map(|d| d.unit_price * Decimal::from(d.quantity)).sum();
        self.discount_total = self.applied_discounts.iter().map(|d| d.discount_amount).sum();
        self.tax_amount     = self.sale_details.iter().map(|d| d.tax_applied).sum();
        self.total_amount   = self.sub_total + self.tax_amount - self.discount_total;
    }
}

#[derive(Debug, Clone)]
pub struct ReturnDetail {
    pub id:         ReturnDetailId,
    pub return_id:  ReturnId,
    pub product_id: Uuid,
    pub quantity:   i32,
    pub reason:     ReturnReason,
    pub restock:    bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Return {
    pub id:             ReturnId,
    pub sale_id:        SaleId,
    pub return_date:    DateTime<Utc>,
    pub employee_id:    Uuid,
    pub customer_id:    Option<Uuid>,
    pub total_refund:   Decimal,
    pub created_at:     DateTime<Utc>,
    pub return_details: Vec<ReturnDetail>,
}

impl Return {
    pub fn create(sale_id: SaleId, employee_id: Uuid, customer_id: Option<Uuid>) -> Self {
        let now = Utc::now();
        Self {
            id: ReturnId::new(),
            sale_id,
            return_date:    now,
            employee_id,
            customer_id,
            total_refund:   Decimal::ZERO,
            created_at:     now,
            return_details: Vec::new(),
        }
    }

    pub fn add_detail(&mut self, product_id: Uuid, quantity: i32, reason: ReturnReason, restock: bool) {
        let det = ReturnDetail {
            id: ReturnDetailId::new(),
            return_id: self.id,
            product_id,
            quantity,
            reason,
            restock,
            created_at: Utc::now(),
        };
        self.return_details.push(det);
    }

    pub fn process(&mut self, total_refund: Decimal) -> Result<(), String> {
        if self.return_details.is_empty() {
            return Err("Cannot process a return with no details.".into());
        }
        self.total_refund = total_refund;
        Ok(())
    }
}

// ── Saga entity ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaOrderItem {
    pub product_id: Uuid,
    pub quantity:   i32,
    pub unit_price: Decimal,
}

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
