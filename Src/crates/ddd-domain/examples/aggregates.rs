use chrono::Utc;
use ddd_domain::{define_aggregate, impl_aggregate, impl_aggregate_events};
use ddd_shared_kernel::{declare_id, DomainEvent};
use serde::{Deserialize, Serialize};

// 1. Declare the identity type
declare_id!(OrderId);

// 2. Define the aggregate struct using the macro
// This automatically adds id, version, created_at, updated_at, and domain_events fields.
define_aggregate!(Order, OrderId, {
    pub customer_id: String,
    pub items: Vec<String>,
    pub status: String,
});

// 3. Implement the AggregateRoot trait
impl_aggregate!(Order, OrderId);

// 4. Add record_event() and clear_events() helpers
impl_aggregate_events!(Order);

// 5. Define a domain event
#[derive(Debug, Serialize, Deserialize)]
struct OrderPlaced {
    order_id: OrderId,
    customer_id: String,
}

impl DomainEvent for OrderPlaced {
    fn event_name(&self) -> &'static str { "order.placed.v1" }
    fn occurred_at(&self) -> chrono::DateTime<chrono::Utc> { chrono::Utc::now() }
    fn as_any(&self) -> &dyn std::any::Any { self }
}

fn main() {
    let order_id = OrderId::new();
    let now = Utc::now();

    let mut order = Order {
        id: order_id,
        version: 0,
        created_at: now,
        updated_at: now,
        domain_events: Vec::new(),
        customer_id: "cust-123".into(),
        items: vec!["apple".into(), "banana".into()],
        status: "Draft".into(),
    };

    println!("Created Order: {:?}", order.id);

    // 6. Record a domain event
    order.record_event(OrderPlaced {
        order_id,
        customer_id: order.customer_id.clone(),
    });

    println!("Pending events: {}", order.domain_events.len());
    for ev in &order.domain_events {
        println!("  - Event: {}", ev.event_name());
    }

    // 7. Clear events
    order.clear_events();
    println!("Events cleared. Count: {}", order.domain_events.len());
}
