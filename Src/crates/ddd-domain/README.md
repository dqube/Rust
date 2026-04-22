# ddd-domain

Pure domain layer: aggregates, entities, value objects, repository ports, specifications, policies, domain services. Depends only on `ddd-shared-kernel`.

## What's inside

| Module | Contents |
|---|---|
| `aggregate` | `Aggregate` trait |
| `entity` | Marker traits |
| `event` | `EventPublisher` — re-exports / helpers over `ddd-shared-kernel::domain_event` |
| `error` | `DomainError` — domain-specific error constructors |
| `repository` | `Repository` — generic repository trait shape |
| `specification` | `Specification`, `SpecificationExt` |
| `policy` | `Policy`, `PolicyChain`, `PolicyViolation` |
| `domain_service` | `DomainService` marker |

## Standalone Examples

For full implementation details, see:
- [`policy_chains.rs`](examples/policy_chains.rs) — Constructing and evaluating business policy chains.
- [`aggregates.rs`](examples/aggregates.rs) — Defining aggregate roots, recorded events, and trait implementations.

## Examples

### Defining an aggregate

```rust
use ddd_shared_kernel::{AggregateRoot, impl_aggregate_events, record_event, AppError, AppResult, DomainEvent};

#[derive(Debug, Default)]
pub struct Order {
    id: OrderId,
    domain_events: Vec<Box<dyn DomainEvent>>,
    status: OrderStatus,
}

impl AggregateRoot for Order {
    type Id = OrderId;
    fn id(&self) -> &Self::Id { &self.id }
}

impl_aggregate_events!(Order);

impl Order {
    pub fn place(id: OrderId) -> Self {
        let mut order = Self { id, ..Default::default() };
        record_event!(order, OrderPlaced { order_id: id });
        order
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

// Compose with boolean logic
let vip_orders = ActiveOrders.and(HighValue(1000.0));
```

### Using policies

```rust
use ddd_domain::policy::{Policy, PolicyChain, PolicyViolation};

struct MaxOrderLimit(u32);
impl Policy<Customer> for MaxOrderLimit {
    fn evaluate(&self, customer: &Customer) -> Result<(), PolicyViolation> {
        if customer.active_order_count() >= self.0 {
            return Err(PolicyViolation::new("Limit exceeded"));
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

## Rules

- **No framework imports.** Not SeaORM, not tonic, not axum, not async-nats.
- Repository traits defined here; concrete SeaORM impls live in `ddd-infrastructure`.
- Aggregates raise events through `record_event!` (from `ddd-shared-kernel`).
- Integration events are not emitted from this layer.
