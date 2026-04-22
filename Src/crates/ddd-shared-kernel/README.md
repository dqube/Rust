# ddd-shared-kernel

Zero-dependency base types and utilities shared by every layer of the DDD stack. Nothing in here depends on other crates in this repo — it is the foundation.

## What's inside

| Module | Contents |
|---|---|
| `error` | `AppError`, `AppResult`, `ValidationFieldError` |
| `id` | `TypedId<T>`, `declare_id!` macro |
| `aggregate` | `AggregateRoot` trait, `impl_aggregate_events!`, `record_event!` |
| `entity` / `value_object` | Marker traits |
| `domain_event` | `DomainEvent`, `DomainEventEnvelope` |
| `integration_event` | `IntegrationEvent`, `IntegrationEventEnvelope` |
| `outbox` | `OutboxMessage`, `OutboxRepository`, `OutboxRelay` (ports) |
| `inbox` | `InboxMessage`, `InboxRepository`, `InboxMessageHandler`, `InboxProcessor` (ports) |
| `dead_letter` | `DeadLetterMessage`, `DeadLetterRepository`, `DeadLetterAlert` |
| `idempotency` | `IdempotencyRecord`, `IdempotencyStore` (port) |
| `saga` | `SagaDefinition`, `SagaInstance`, `SagaStatus` |
| `pagination` | `Page<T>`, `PageRequest` |
| `validation` | Fluent validation API, `validate!`, `validate_all!` |
| `jwt` | `JwtValidator`, `StandardClaims` (feature `jwt`) |

## Standalone Examples

For full implementation details, see:
- [`jwt_usage.rs`](examples/jwt_usage.rs) — HS256 validation, custom claims, and leeway configuration.
- [`saga_definition.rs`](examples/saga_definition.rs) — Constructing complex multi-step saga workflows.

## Examples

### Typed IDs

```rust
use ddd_shared_kernel::declare_id;

declare_id!(OrderId);
declare_id!(CustomerId);

let order_id = OrderId::new();       // random UUIDv7
let parsed: OrderId = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
println!("{}", order_id);            // prints the UUID
```

### Error handling

```rust
use ddd_shared_kernel::{AppError, AppResult};

fn find_order(id: &str) -> AppResult<Order> {
    let order = repo.find(id)
        .ok_or_else(|| AppError::not_found("Order", id))?;
    Ok(order)
}

// Validation errors with field-level detail
fn validate_input(email: &str) -> AppResult<()> {
    if email.is_empty() {
        return Err(AppError::validation("email", "must not be empty"));
    }
    Ok(())
}
```

### Aggregate macros

```rust
use ddd_shared_kernel::{AggregateRoot, impl_aggregate_events, record_event};

#[derive(Debug, Default)]
struct Order {
    id: OrderId,
    domain_events: Vec<Box<dyn DomainEvent>>,
    // ...
}

impl AggregateRoot for Order {
    type Id = OrderId;
    fn id(&self) -> &Self::Id { &self.id }
}

impl_aggregate_events!(Order);

// Inside a domain method:
record_event!(self, OrderPlaced { order_id: self.id });
```

### Integration events + outbox

```rust
use ddd_shared_kernel::{IntegrationEvent, OutboxMessage};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderPlacedIntegration { order_id: Uuid }

impl IntegrationEvent for OrderPlacedIntegration {
    fn event_type(&self) -> &str { "order.placed" }
}

// Inside a command handler (same transaction as aggregate save):
let msg = OutboxMessage::new("order.placed", &OrderPlacedIntegration { order_id })?;
outbox_repo.store(msg).await?;
```

### Saga definitions

```rust
use ddd_shared_kernel::saga::{SagaDefinition, SagaStepDefinition};

let definition = SagaDefinition {
    saga_type: "order-saga".into(),
    steps: vec![
        SagaStepDefinition {
            name: "reserve-inventory".into(),
            action_event_type: "inventory.reserve".into(),
            compensation_event_type: "inventory.release".into(),
            ..
        },
    ],
};
```

### Fluent validation

```rust
use ddd_shared_kernel::{validate, validate_all};
use ddd_shared_kernel::validation::ValidationResult;

let result: ValidationResult = validate_all!(
    validate!(email, "email").not_empty().email().into(),
    validate!(&age, "age").positive().in_range(18, 120).into()
);
// Returns Err(AppError::ValidationBatch(...)) if any field fails
result.into_app_error()?;
```

## Rules

- **Zero dependencies** on other crates in this repository.
- No framework types (no SeaORM, tonic, axum, async-nats).
- Ports (traits) for outbox/inbox/dead-letter/idempotency/saga live here; implementations live in `ddd-infrastructure`.
