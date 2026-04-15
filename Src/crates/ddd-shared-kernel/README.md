# ddd-shared-kernel

Zero-dependency base types and utilities shared by every layer of the DDD stack. Nothing in here depends on other crates in this repo — it is the foundation.

## What's inside

| Module | Contents |
|---|---|
| `error` | `AppError`, `AppResult`, `ValidationFieldError` |
| `id` | `TypedId<T>`, `declare_id!` macro |
| `aggregate` | `AggregateRoot` trait, `impl_aggregate_root!`, `record_event!` |
| `entity` / `value_object` | Marker traits + `impl_value_object!` |
| `domain_event` | `DomainEvent`, `DomainEventEnvelope`, `DomainEventDispatcher` |
| `integration_event` | `IntegrationEvent`, `IntegrationEventEnvelope`, `IntegrationEventPublisher` |
| `outbox` | `OutboxMessage`, `OutboxRepository`, `OutboxRelay` (ports) |
| `inbox` | `InboxMessage`, `InboxRepository`, `InboxMessageHandler`, `InboxProcessor` (ports) |
| `dead_letter` | `DeadLetterMessage`, `DeadLetterOrigin`, `DeadLetterRepository`, `DeadLetterAlert`, `LogDeadLetterAlert` |
| `idempotency` | `IdempotencyRecord`, `IdempotencyStore` (port) |
| `saga` | `SagaDefinition`, `SagaInstance`, `SagaOrchestrator`, `SagaInstanceRepository` (ports + types) |
| `pagination` | `Page<T>`, `PageRequest` |
| `validation` | Fluent validation API, `validate!`, `validate_all!` |

## Features

- `validation` — enables `validator` + `regex` dependencies for fluent rules.
- `grpc` — enables a `tonic::Status` conversion for `AppError`.

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
        return Err(AppError::validation_field("email", "must not be empty"));
    }
    Ok(())
}
```

### Domain events

```rust
use ddd_shared_kernel::{DomainEvent, AggregateRoot, record_event, impl_aggregate_root};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderPlaced { order_id: Uuid, total: f64 }

impl DomainEvent for OrderPlaced {
    fn event_type(&self) -> &str { "OrderPlaced" }
}
```

### Integration events + outbox

```rust
use ddd_shared_kernel::{IntegrationEvent, OutboxMessage};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderPlacedIntegration { order_id: Uuid }

impl IntegrationEvent for OrderPlacedIntegration {
    fn event_type(&self) -> &str { "order.placed" }
    fn subject(&self) -> &str { "orders" }
}

// Inside a command handler (same transaction as aggregate save):
let msg = OutboxMessage::new("order.placed", &OrderPlacedIntegration { order_id })?;
outbox_repo.store(msg).await?;
```

### Dead-letter queue

```rust
use ddd_shared_kernel::dead_letter::{DeadLetterAlert, LogDeadLetterAlert};

// OutboxRelay and InboxProcessor automatically move failed messages
// to the dead-letter store after max_attempts:
let relay = OutboxRelay::new(
    outbox_repo,
    publisher,
    dead_letter_repo,
    Arc::new(LogDeadLetterAlert),  // logs alerts; implement DeadLetterAlert for custom behaviour
    5,                              // max_attempts
);
```

### Idempotency store

```rust
use ddd_shared_kernel::idempotency::{IdempotencyStore, IdempotencyRecord};

// Port — implementations live in ddd-infrastructure
#[async_trait]
pub trait IdempotencyStore: Send + Sync {
    async fn get(&self, key: &str) -> AppResult<Option<IdempotencyRecord>>;
    async fn store(&self, record: IdempotencyRecord) -> AppResult<()>;
    async fn delete(&self, key: &str) -> AppResult<()>;
}
```

### Saga types

```rust
use ddd_shared_kernel::saga::{SagaDefinition, SagaStepDefinition, SagaInstance, SagaStatus};

let definition = SagaDefinition::new("order-saga", vec![
    SagaStepDefinition {
        name: "reserve-inventory".into(),
        action_event_type: "inventory.reserve".into(),
        action_subject: "inventory".into(),
        compensation_event_type: "inventory.release".into(),
        compensation_subject: "inventory".into(),
    },
    SagaStepDefinition {
        name: "charge-payment".into(),
        action_event_type: "payment.charge".into(),
        action_subject: "payments".into(),
        compensation_event_type: "payment.refund".into(),
        compensation_subject: "payments".into(),
    },
]);
```

### Fluent validation

```rust
use ddd_shared_kernel::{validate, validate_all};
use ddd_shared_kernel::validation::ValidationResult;

let result: ValidationResult = validate_all!(
    validate!(email, "email").not_empty().email().into(),
    validate!(&age, "age").positive().in_range(18, 120).into(),
    validate!(name, "name").not_empty().min_length(2).max_length(100).into()
);
// result is Err(AppError::ValidationBatch(...)) if any field fails
```

### Pagination

```rust
use ddd_shared_kernel::{Page, PageRequest};

let request = PageRequest::new(0, 20);  // page 0, size 20
let page: Page<OrderDto> = Page {
    content: vec![/* items */],
    total_elements: 100,
    total_pages: 5,
    page: 0,
    size: 20,
};
```

## Rules

- No framework types (no SeaORM, tonic, axum, async-nats).
- Ports (traits) for outbox/inbox/dead-letter/idempotency/saga live here; implementations live in `ddd-infrastructure`.
- Every outer crate imports from here; keep additions backward-compatible.
