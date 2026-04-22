//! Command definitions for the Order bounded context.

use crate::domain::events::OrderId;
use crate::domain::value_objects::OrderItem;

// ─── CreateOrder ─────────────────────────────────────────────────────────────

pub struct CreateOrder {
    pub customer_id: String,
    pub items: Vec<OrderItem>,
}

ddd_application::impl_command!(CreateOrder, OrderId);

// ─── ConfirmOrder ────────────────────────────────────────────────────────────

pub struct ConfirmOrder {
    pub order_id: OrderId,
}

ddd_application::impl_command!(ConfirmOrder, ());

// ─── CancelOrder ─────────────────────────────────────────────────────────────

pub struct CancelOrder {
    pub order_id: OrderId,
    pub reason: String,
}

ddd_application::impl_command!(CancelOrder, ());
