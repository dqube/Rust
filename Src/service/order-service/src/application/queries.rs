//! Query definitions for the Order bounded context.

use crate::domain::aggregate::Order;
use crate::domain::events::OrderId;
use ddd_shared_kernel::Page;

// ─── GetOrder ────────────────────────────────────────────────────────────────

pub struct GetOrder {
    pub order_id: OrderId,
}

ddd_application::impl_query!(GetOrder, Option<Order>);

// ─── ListOrders ──────────────────────────────────────────────────────────────

pub struct ListOrders {
    pub page: u32,
    pub per_page: u32,
}

ddd_application::impl_query!(ListOrders, Page<Order>);
