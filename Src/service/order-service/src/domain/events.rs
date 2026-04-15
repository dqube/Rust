//! Domain events raised by the Order aggregate.

use chrono::{DateTime, Utc};
use ddd_shared_kernel::{declare_id, DomainEvent};
use serde::{Deserialize, Serialize};
use std::any::Any;

use super::value_objects::{Money, OrderItem};

declare_id!(OrderId);

// ─── OrderCreated ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCreated {
    pub order_id: OrderId,
    pub customer_id: String,
    pub items: Vec<OrderItem>,
    pub total_amount: Money,
    pub occurred_at: DateTime<Utc>,
}

impl DomainEvent for OrderCreated {
    fn event_name(&self) -> &'static str {
        "order.created"
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ─── OrderConfirmed ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderConfirmed {
    pub order_id: OrderId,
    pub occurred_at: DateTime<Utc>,
}

impl DomainEvent for OrderConfirmed {
    fn event_name(&self) -> &'static str {
        "order.confirmed"
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ─── OrderCancelled ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCancelled {
    pub order_id: OrderId,
    pub reason: String,
    pub occurred_at: DateTime<Utc>,
}

impl DomainEvent for OrderCancelled {
    fn event_name(&self) -> &'static str {
        "order.cancelled"
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
