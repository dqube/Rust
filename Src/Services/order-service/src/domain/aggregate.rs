//! Order aggregate root.

use chrono::Utc;
use ddd_domain::{define_aggregate, impl_aggregate, impl_aggregate_events};
use ddd_shared_kernel::AppError;

use super::events::{OrderCancelled, OrderConfirmed, OrderCreated, OrderId};
use super::value_objects::{Money, OrderItem, OrderStatus};

define_aggregate!(Order, OrderId, {
    pub customer_id: String,
    pub items: Vec<OrderItem>,
    pub total_amount: Money,
    pub status: OrderStatus,
});

impl_aggregate!(Order, OrderId);
impl_aggregate_events!(Order);

impl Order {
    /// Create a new order in Draft status.
    pub fn create(customer_id: impl Into<String>, items: Vec<OrderItem>) -> Result<Self, AppError> {
        if items.is_empty() {
            return Err(AppError::validation("items", "Order must have at least one item"));
        }

        let total = items
            .iter()
            .fold(Money::zero(), |acc, item| acc.add(&item.line_total()));

        let now = Utc::now();
        let id = OrderId::new();

        let mut order = Self {
            id,
            version: 0,
            created_at: now,
            updated_at: now,
            domain_events: Vec::new(),
            customer_id: customer_id.into(),
            items: items.clone(),
            total_amount: total.clone(),
            status: OrderStatus::Placed,
        };

        order.record_event(OrderCreated {
            order_id: id,
            customer_id: order.customer_id.clone(),
            items,
            total_amount: total,
            occurred_at: now,
        });

        Ok(order)
    }

    /// Confirm the order, transitioning from Placed → Confirmed.
    pub fn confirm(&mut self) -> Result<(), AppError> {
        match self.status {
            OrderStatus::Placed => {
                self.status = OrderStatus::Confirmed;
                self.updated_at = Utc::now();

                self.record_event(OrderConfirmed {
                    order_id: self.id,
                    occurred_at: self.updated_at,
                });

                Ok(())
            }
            _ => Err(AppError::business_rule(format!(
                "Cannot confirm order in '{}' status",
                self.status
            ))),
        }
    }

    /// Cancel the order (from Placed or Confirmed).
    pub fn cancel(&mut self, reason: impl Into<String>) -> Result<(), AppError> {
        match self.status {
            OrderStatus::Cancelled => Err(AppError::business_rule("Order is already cancelled")),
            _ => {
                let reason = reason.into();
                self.status = OrderStatus::Cancelled;
                self.updated_at = Utc::now();

                self.record_event(OrderCancelled {
                    order_id: self.id,
                    reason,
                    occurred_at: self.updated_at,
                });

                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_items() -> Vec<OrderItem> {
        vec![
            OrderItem::new("SKU-001", 2, Money::from_f64(10.00)),
            OrderItem::new("SKU-002", 1, Money::from_f64(25.50)),
        ]
    }

    #[test]
    fn create_computes_total() {
        let order = Order::create("cust-1", sample_items()).unwrap();
        // 2 * 10.00 + 1 * 25.50 = 45.50
        assert_eq!(order.total_amount.cents, 4550);
        assert_eq!(order.status, OrderStatus::Placed);
        assert_eq!(order.domain_events.len(), 1);
    }

    #[test]
    fn create_empty_items_fails() {
        let result = Order::create("cust-1", vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn confirm_placed_order() {
        let mut order = Order::create("cust-1", sample_items()).unwrap();
        order.confirm().unwrap();
        assert_eq!(order.status, OrderStatus::Confirmed);
        assert_eq!(order.domain_events.len(), 2);
    }

    #[test]
    fn confirm_cancelled_order_fails() {
        let mut order = Order::create("cust-1", sample_items()).unwrap();
        order.cancel("changed mind").unwrap();
        assert!(order.confirm().is_err());
    }

    #[test]
    fn cancel_order() {
        let mut order = Order::create("cust-1", sample_items()).unwrap();
        order.cancel("test reason").unwrap();
        assert_eq!(order.status, OrderStatus::Cancelled);
    }

    #[test]
    fn cancel_already_cancelled_fails() {
        let mut order = Order::create("cust-1", sample_items()).unwrap();
        order.cancel("once").unwrap();
        assert!(order.cancel("twice").is_err());
    }
}
