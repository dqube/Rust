use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Published ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleCreatedIntegrationEvent {
    pub sale_id:          Uuid,
    pub store_id:         i32,
    pub employee_id:      Uuid,
    pub customer_id:      Option<Uuid>,
    pub total_amount:     Decimal,
    pub transaction_time: String,
}
impl SaleCreatedIntegrationEvent { pub const TOPIC: &'static str = "v1.sales.sale.created"; }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleCompletedIntegrationEvent {
    pub sale_id:          Uuid,
    pub total_amount:     Decimal,
    pub transaction_time: String,
}
impl SaleCompletedIntegrationEvent { pub const TOPIC: &'static str = "v1.sales.sale.completed"; }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleCancelledIntegrationEvent {
    pub sale_id:      Uuid,
    pub reason:       String,
    pub cancelled_at: String,
}
impl SaleCancelledIntegrationEvent { pub const TOPIC: &'static str = "v1.sales.sale.cancelled"; }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnCreatedIntegrationEvent {
    pub return_id:    Uuid,
    pub sale_id:      Uuid,
    pub employee_id:  Uuid,
    pub customer_id:  Option<Uuid>,
    pub return_date:  String,
}
impl ReturnCreatedIntegrationEvent { pub const TOPIC: &'static str = "v1.sales.return.created"; }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnProcessedIntegrationEvent {
    pub return_id:    Uuid,
    pub sale_id:      Uuid,
    pub total_refund: Decimal,
}
impl ReturnProcessedIntegrationEvent { pub const TOPIC: &'static str = "v1.sales.return.processed"; }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderLineItemAddedIntegrationEvent {
    pub order_id:     Uuid,
    pub line_item_id: Uuid,
    pub product_id:   Uuid,
    pub quantity:     i32,
    pub unit_price:   Decimal,
    pub added_at:     String,
}
impl OrderLineItemAddedIntegrationEvent { pub const TOPIC: &'static str = "v1.sales.order.line-item-added"; }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaItem {
    pub product_id: Uuid,
    pub quantity:   i32,
    pub unit_price: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPlacedIntegrationEvent {
    pub order_id:     Uuid,
    pub order_number: String,
    pub customer_id:  Uuid,
    pub store_id:     i32,
    pub sub_total:    Decimal,
    pub tax:          Decimal,
    pub total:        Decimal,
    pub currency:     String,
    pub items:        Vec<SagaItem>,
    pub placed_at:    String,
}
impl OrderPlacedIntegrationEvent {
    pub const TOPIC: &'static str = "v1.sales.order.placed";
    pub const GROUP: &'static str = "sales-service";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderConfirmedIntegrationEvent {
    pub order_id:     Uuid,
    pub order_number: String,
    pub customer_id:  Uuid,
    pub store_id:     i32,
    pub items:        Vec<SagaItem>,
    pub total:        Decimal,
    pub confirmed_at: String,
}
impl OrderConfirmedIntegrationEvent { pub const TOPIC: &'static str = "v1.sales.order.confirmed"; }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCancelledIntegrationEvent {
    pub order_id:     Uuid,
    pub order_number: String,
    pub customer_id:  Uuid,
    pub store_id:     i32,
    pub items:        Vec<SagaItem>,
    pub reason:       String,
    pub cancelled_at: String,
}
impl OrderCancelledIntegrationEvent { pub const TOPIC: &'static str = "v1.sales.order.cancelled"; }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRefundRequestedIntegrationEvent {
    pub order_id:     Uuid,
    pub customer_id:  Uuid,
    pub total_amount: Decimal,
    pub reason:       String,
}
impl OrderRefundRequestedIntegrationEvent { pub const TOPIC: &'static str = "v1.sales.order.refund-requested"; }

// ── Subscribed ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockReservedIntegrationEvent {
    pub order_id:       Uuid,
    pub reservation_id: Uuid,
}
impl StockReservedIntegrationEvent {
    pub const TOPIC: &'static str = "v1.inventory.stock.reserved";
    pub const GROUP: &'static str = "sales-service";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockReservationFailedIntegrationEvent {
    pub order_id: Uuid,
    pub reason:   String,
}
impl StockReservationFailedIntegrationEvent {
    pub const TOPIC: &'static str = "v1.inventory.stock.reservation-failed";
    pub const GROUP: &'static str = "sales-service";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentCapturedIntegrationEvent {
    pub order_id:   Uuid,
    pub payment_id: Uuid,
}
impl PaymentCapturedIntegrationEvent {
    pub const TOPIC: &'static str = "v1.payment.payment.captured";
    pub const GROUP: &'static str = "sales-service";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentFailedIntegrationEvent {
    pub order_id: Uuid,
    pub reason:   String,
}
impl PaymentFailedIntegrationEvent {
    pub const TOPIC: &'static str = "v1.payment.payment.failed";
    pub const GROUP: &'static str = "sales-service";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInitiationFailedIntegrationEvent {
    pub order_id: Uuid,
    pub reason:   String,
}
impl PaymentInitiationFailedIntegrationEvent {
    pub const TOPIC: &'static str = "v1.payment.payment.initiation-failed";
    pub const GROUP: &'static str = "sales-service";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRefundedIntegrationEvent {
    pub sale_id: Uuid,
    pub reason:  String,
}
impl PaymentRefundedIntegrationEvent {
    pub const TOPIC: &'static str = "v1.payment.payment.refunded";
    pub const GROUP: &'static str = "sales-service";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionAppliedIntegrationEvent {
    pub sale_id:         Uuid,
    pub campaign_id:     Uuid,
    pub rule_id:         Uuid,
    pub discount_amount: Decimal,
}
impl PromotionAppliedIntegrationEvent {
    pub const TOPIC: &'static str = "v1.promotion.promotion.applied";
    pub const GROUP: &'static str = "sales-service";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerDeletedIntegrationEvent {
    pub customer_id: Uuid,
}
impl CustomerDeletedIntegrationEvent {
    pub const TOPIC: &'static str = "v1.customer.customer.deleted";
    pub const GROUP: &'static str = "sales-service";
}
