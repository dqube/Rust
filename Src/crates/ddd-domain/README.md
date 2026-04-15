# ddd-domain

Pure domain layer: aggregates, entities, value objects, repository ports, specifications, policies, domain services. Depends only on `ddd-shared-kernel`.

## What's inside

| Module | Contents |
|---|---|
| `aggregate` | `Aggregate` trait |
| `entity` | `Entity` trait |
| `event` | `EventPublisher`, `EventRouter` — re-exports / helpers over `ddd-shared-kernel::domain_event` |
| `error` | `DomainError` — domain-specific error constructors |
| `repository` | `Repository` — generic repository trait shape |
| `specification` | `Specification`, `SpecificationExt`, `AndSpec`, `OrSpec`, `NotSpec`, `ClosureSpec` |
| `policy` | `Policy`, `PolicyChain`, `PolicyViolation` |
| `domain_service` | `DomainService`, `DomainServiceFor` |
| `macros` | Helper macros for domain patterns |

## Features

- `tracing` — instrument domain methods with `#[tracing::instrument]`.

## Examples

### Defining an aggregate

```rust
use chrono::{DateTime, Utc};
use ddd_shared_kernel::{AggregateRoot, declare_id, record_event, impl_aggregate_root, AppError, AppResult, DomainEvent};

declare_id!(OrderId);

#[derive(Debug)]
pub struct Order {
    id: OrderId,
    version: u64,
    updated_at: DateTime<Utc>,
    domain_events: Vec<Box<dyn DomainEvent>>,
    status: OrderStatus,
    items: Vec<LineItem>,
}

impl_aggregate_root!(Order, OrderId);

impl Order {
    pub fn place(id: OrderId, items: Vec<LineItem>) -> Self {
        let order = Self {
            id,
            version: 0,
            updated_at: Utc::now(),
            domain_events: vec![],
            status: OrderStatus::Placed,
            items,
        };
        record_event!(order, OrderPlaced { order_id: id.into() });
        order
    }

    pub fn cancel(&mut self) -> AppResult<()> {
        if self.status == OrderStatus::Shipped {
            return Err(AppError::business_rule("Cannot cancel a shipped order"));
        }
        self.status = OrderStatus::Cancelled;
        record_event!(self, OrderCancelled { order_id: self.id.into() });
        Ok(())
    }

    pub fn ship(&mut self) -> AppResult<()> {
        if self.status != OrderStatus::Placed {
            return Err(AppError::business_rule("Only placed orders can be shipped"));
        }
        self.status = OrderStatus::Shipped;
        record_event!(self, OrderShipped { order_id: self.id.into() });
        Ok(())
    }
}
```

### Defining a repository port

```rust
use ddd_shared_kernel::AppResult;
use async_trait::async_trait;

#[async_trait]
pub trait OrderRepository: Send + Sync {
    async fn find_by_id(&self, id: &OrderId) -> AppResult<Option<Order>>;
    async fn save(&self, order: &Order) -> AppResult<()>;
    async fn delete(&self, id: &OrderId) -> AppResult<()>;
}
```

### Using specifications

```rust
use ddd_domain::specification::{Specification, SpecificationExt};

struct ActiveOrders;
impl Specification<Order> for ActiveOrders {
    fn is_satisfied_by(&self, order: &Order) -> bool {
        order.status() == OrderStatus::Active
    }
}

struct HighValue(f64);
impl Specification<Order> for HighValue {
    fn is_satisfied_by(&self, order: &Order) -> bool {
        order.total() > self.0
    }
}

// Compose with boolean logic
let vip_orders = ActiveOrders.and(HighValue(1000.0));
let needs_review = ActiveOrders.and(HighValue(5000.0).not());

assert!(vip_orders.is_satisfied_by(&expensive_active_order));
```

### Using policies

```rust
use ddd_domain::policy::{Policy, PolicyChain, PolicyViolation};

struct MaxOrderLimit(u32);
impl Policy<Customer> for MaxOrderLimit {
    fn evaluate(&self, customer: &Customer) -> Result<(), PolicyViolation> {
        if customer.active_order_count() >= self.0 {
            return Err(PolicyViolation::new("Maximum order limit exceeded"));
        }
        Ok(())
    }
}

struct AccountInGoodStanding;
impl Policy<Customer> for AccountInGoodStanding {
    fn evaluate(&self, customer: &Customer) -> Result<(), PolicyViolation> {
        if customer.has_overdue_payments() {
            return Err(PolicyViolation::new("Account has overdue payments"));
        }
        Ok(())
    }
}

// Chain policies — all must pass
let policies = PolicyChain::new(vec![
    Box::new(MaxOrderLimit(100)),
    Box::new(AccountInGoodStanding),
]);
policies.evaluate(&customer)?;
```

### Using domain services

```rust
use ddd_domain::domain_service::DomainService;

struct PricingService {
    discount_repo: Arc<dyn DiscountRepository>,
}

impl DomainService for PricingService {}

impl PricingService {
    pub async fn calculate_price(&self, order: &Order, customer: &Customer) -> AppResult<Money> {
        let base = order.subtotal();
        let discount = self.discount_repo.find_for_customer(customer.id()).await?;
        Ok(base.apply_discount(discount))
    }
}
```

### Using closure specifications (ad hoc)

```rust
use ddd_domain::specification::ClosureSpec;

let recent = ClosureSpec::new(|order: &Order| {
    order.created_at() > Utc::now() - Duration::days(30)
});
let recent_high_value = recent.and(HighValue(500.0));
```

## Rules

- **No framework imports.** Not SeaORM, not tonic, not axum, not async-nats.
- Repository traits defined here; concrete SeaORM impls live in `ddd-infrastructure`.
- Aggregates raise events through `record_event!` (from `ddd-shared-kernel`).
- Integration events are not emitted from this layer. Application handlers translate domain events → integration events via the outbox.

## Typical addition

Add a file per aggregate: the struct, its domain methods, the repository trait (port), and any invariants. No DTOs, no mapping code.
