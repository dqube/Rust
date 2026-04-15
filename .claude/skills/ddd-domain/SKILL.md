---
name: ddd-domain
description: Guidance for the ddd-domain crate — pure domain layer with aggregates, entities, value objects, repository ports, specifications, policies, and domain services. Use when modelling domain logic.
---

# ddd-domain

Pure domain layer. Depends only on `ddd-shared-kernel`. No framework imports (no SeaORM, tonic, axum, async-nats). No DTOs, no mapping code.

## Modules

| Module | Key types |
|--------|-----------|
| `aggregate` | `Aggregate` trait |
| `entity` | `Entity` trait |
| `repository` | `Repository` trait (generic port) |
| `specification` | `Specification`, `SpecificationExt`, `AndSpec`, `OrSpec`, `NotSpec`, `ClosureSpec` |
| `policy` | `Policy`, `PolicyChain`, `PolicyViolation` |
| `domain_service` | `DomainService`, `DomainServiceFor` |
| `event` | `EventPublisher`, `EventRouter` (re-exports/helpers over shared-kernel events) |
| `error` | `DomainError` — domain-specific error constructors |
| `macros` | Helper macros for domain patterns |

## Feature flags

- `tracing` — instrument domain methods with `#[tracing::instrument]`.

## Recipes

### Adding an aggregate
```rust
use ddd_shared_kernel::{AggregateRoot, declare_id, record_event};

declare_id!(OrderId);

pub struct Order {
    id: OrderId,
    status: OrderStatus,
    events: Vec<Box<dyn DomainEvent>>,
}

impl_aggregate_root!(Order, OrderId);

impl Order {
    pub fn place(id: OrderId, items: Vec<LineItem>) -> Self {
        let order = Self { id, status: OrderStatus::Placed, events: vec![] };
        record_event!(order, OrderPlaced { order_id: id.into() });
        order
    }

    pub fn cancel(&mut self) -> AppResult<()> {
        if self.status == OrderStatus::Shipped {
            return Err(AppError::business_rule("Cannot cancel shipped order"));
        }
        self.status = OrderStatus::Cancelled;
        record_event!(self, OrderCancelled { order_id: self.id.into() });
        Ok(())
    }
}
```

### Adding a repository port
Define the trait next to the aggregate, not in infrastructure:
```rust
#[async_trait]
pub trait OrderRepository: Send + Sync {
    async fn find_by_id(&self, id: &OrderId) -> AppResult<Option<Order>>;
    async fn save(&self, order: &Order) -> AppResult<()>;
    async fn delete(&self, id: &OrderId) -> AppResult<()>;
    async fn find_by_spec(&self, spec: &dyn Specification<Order>) -> AppResult<Vec<Order>>;
}
```

### Using specifications
```rust
use ddd_domain::specification::{Specification, SpecificationExt};

struct ActiveOrders;
impl Specification<Order> for ActiveOrders {
    fn is_satisfied_by(&self, order: &Order) -> bool {
        order.status == OrderStatus::Active
    }
}

struct HighValue;
impl Specification<Order> for HighValue {
    fn is_satisfied_by(&self, order: &Order) -> bool {
        order.total() > 1000.0
    }
}

// Compose specifications
let spec = ActiveOrders.and(HighValue);
let orders = repo.find_by_spec(&spec).await?;
```

### Using policies
```rust
use ddd_domain::policy::{Policy, PolicyChain, PolicyViolation};

struct MaxOrderLimit(u32);
impl Policy<Customer> for MaxOrderLimit {
    fn evaluate(&self, customer: &Customer) -> Result<(), PolicyViolation> {
        if customer.order_count() >= self.0 {
            return Err(PolicyViolation::new("Order limit exceeded"));
        }
        Ok(())
    }
}

let policy_chain = PolicyChain::new(vec![
    Box::new(MaxOrderLimit(100)),
    Box::new(AccountInGoodStanding),
]);
policy_chain.evaluate(&customer)?;
```

## Rules

- **No framework imports.** Repository traits defined here; SeaORM impls live in `ddd-infrastructure`.
- Aggregates raise events through `record_event!` (from `ddd-shared-kernel`).
- Integration events are NOT emitted from this layer — application handlers translate domain events → integration events via the outbox.
- One file per aggregate: struct, domain methods, repository trait, invariants.
