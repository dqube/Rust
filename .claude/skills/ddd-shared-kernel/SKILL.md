---
name: ddd-shared-kernel
description: Guidance for the ddd-shared-kernel crate — zero-dependency base types shared by every DDD layer. Use when adding error types, ids, pagination, outbox/inbox ports, dead-letter queue, idempotency, saga types, validation, or domain/integration event primitives.
---

# ddd-shared-kernel

Zero-dependency foundation crate. Every other crate in the stack depends on this. No framework types allowed (no SeaORM, tonic, axum, async-nats).

## Modules

| Module | Key types |
|--------|-----------|
| `error` | `AppError` (Validation, ValidationBatch, NotFound, Conflict, Unauthorized, Forbidden, BusinessRule, Internal, Database, Serialization), `AppResult<T>`, `ValidationFieldError` |
| `id` | `TypedId<T>`, `declare_id!` macro |
| `aggregate` | `AggregateRoot` trait, `impl_aggregate_root!`, `record_event!` |
| `entity` / `value_object` | `Entity`, `ValueObject` marker traits, `impl_value_object!` |
| `domain_event` | `DomainEvent`, `DomainEventEnvelope`, `DomainEventDispatcher` |
| `integration_event` | `IntegrationEvent`, `IntegrationEventEnvelope`, `IntegrationEventPublisher` |
| `outbox` | `OutboxMessage`, `OutboxRepository` trait, `OutboxRelay` |
| `inbox` | `InboxMessage`, `InboxRepository` trait, `InboxMessageHandler` trait, `InboxProcessor` |
| `dead_letter` | `DeadLetterMessage`, `DeadLetterOrigin`, `DeadLetterRepository` trait, `DeadLetterAlert` trait, `LogDeadLetterAlert` |
| `idempotency` | `IdempotencyRecord`, `IdempotencyStore` trait |
| `saga` | `SagaStatus`, `SagaStepStatus`, `SagaStepDefinition`, `SagaDefinition`, `SagaStepState`, `SagaInstance`, `SagaInstanceRepository` trait, `SagaOrchestrator` trait |
| `pagination` | `Page<T>`, `PageRequest` |
| `validation` | `FluentValidator`, `ValidationError`, `ValidationResult`, `ValidationRule`, `validate!`, `validate_all!` |

## Feature flags

- `validation` — enables `validator` + `regex` for fluent validation rules.
- `grpc` — enables `tonic::Status` conversion for `AppError`.

## Recipes

### Adding a new error variant
Add the variant to `AppError` in `error.rs`. Implement `http_status_code()` mapping. If exposing via gRPC, update the `#[cfg(feature = "grpc")]` impl block.

### Declaring a typed ID
```rust
use ddd_shared_kernel::declare_id;
declare_id!(OrderId);
```
Produces a newtype over `uuid::Uuid` with `Display`, `FromStr`, `Serialize`, `Deserialize`.

### Defining a domain event
```rust
use ddd_shared_kernel::DomainEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPlaced { pub order_id: Uuid, pub total: f64 }

impl DomainEvent for OrderPlaced {
    fn event_type(&self) -> &str { "OrderPlaced" }
}
```

### Defining an integration event
```rust
use ddd_shared_kernel::IntegrationEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPlacedIntegration { pub order_id: Uuid }

impl IntegrationEvent for OrderPlacedIntegration {
    fn event_type(&self) -> &str { "order.placed" }
    fn subject(&self) -> &str { "orders" }
}
```

### Using fluent validation
```rust
use ddd_shared_kernel::validation::FluentValidator;
use ddd_shared_kernel::validate;

let result = validate! {
    "email" => FluentValidator::new(&email).not_empty().email(),
    "age"   => FluentValidator::new(&age).range(18..=120),
};
```

### Using the outbox pattern
Inside a command handler, within a UnitOfWork transaction:
```rust
// Persist aggregate changes
repo.save(&order).await?;
// Append integration event to outbox in same transaction
let msg = OutboxMessage::new("order.placed", &OrderPlacedIntegration { order_id })?;
outbox_repo.store(msg).await?;
// OutboxRelay picks it up and publishes to NATS
```

## Rules

- No framework types. Ports (traits) only — implementations live in `ddd-infrastructure`.
- Every outer crate imports from here; keep additions backward-compatible.
- OutboxRelay and InboxProcessor require dead-letter repository + alert for resilience.
