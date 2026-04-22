# ddd-application

Application-layer building blocks: CQRS dispatch, unit-of-work, validation, use cases, ports (clock, id generator, event publisher), domain-event handler registry, idempotency decorator, saga orchestrator, and the **mediator** that fans out commands / queries / domain events.

## What's inside

| Module | Contents |
|---|---|
| `cqrs` | `Command`, `Query`, `CommandHandler`, `QueryHandler` |
| `mediator` | `Mediator` facade (`send` / `query` / `publish`) + inventory registration |
| `event_handling` | `DomainEventHandler`, `EventHandlerRegistry` |
| `unit_of_work` | `UnitOfWork`, `UnitOfWorkFactory` |
| `idempotency` | `IdempotentCommand` trait, `IdempotentCommandHandler` decorator |
| `saga` | `DefaultSagaOrchestrator` (state machine), `SagaDefinitionRegistry` |
| `macros` | `register_command_handler!`, `register_query_handler!`, `register_event_handler!` |

## Standalone Examples

For full implementation details, see:
- [`mediator_registration.rs`](examples/mediator_registration.rs) — Comparison of manual vs. inventory-based handler registration.
- [`saga_orchestration.rs`](examples/saga_orchestration.rs) — Full lifecycle of a Saga instance using in-memory persistence.
- [`idempotent_handler.rs`](examples/idempotent_handler.rs) — Decorating command handlers with idempotency checks.

## The Mediator

`Mediator` is the single dispatch entry point:

```rust
mediator.send(CreateOrder { ... }).await?;        // command, 1:1 handler
mediator.query(GetOrder { id }).await?;           // query, 1:1 handler
mediator.publish(event, agg_id, "Order", 1).await?;  // events, 1:N
```

### Self-registration

Handlers register themselves at link time through the `inventory` crate:

```rust
register_command_handler!(CreateOrder, AppDeps, |d: &AppDeps| {
    CreateOrderHandler::new(d.repo.clone(), d.outbox.clone())
});
```

Then at startup:

```rust
let mediator = Mediator::from_inventory(&deps);
```

## Examples

### Full command handler with outbox

```rust
use ddd_application::{Command, CommandHandler};
use ddd_shared_kernel::{AppResult, OutboxMessage, OutboxRepository};

pub struct CreateOrder { pub sku: String }
impl Command for CreateOrder { type Response = uuid::Uuid; }

pub struct CreateOrderHandler {
    repo: Arc<dyn OrderRepository>,
    outbox: Arc<dyn OutboxRepository>,
    uow: Arc<dyn UnitOfWorkFactory>,
}

#[async_trait]
impl CommandHandler<CreateOrder> for CreateOrderHandler {
    async fn handle(&self, cmd: CreateOrder) -> AppResult<Uuid> {
        let order = Order::place(OrderId::new());
        let id = *order.id();

        let mut uow = self.uow.begin().await?;
        self.repo.save(&order).await?;

        let msg = OutboxMessage::new("order.placed", &OrderPlacedIntegration { order_id: id })?;
        self.outbox.store(msg).await?;

        uow.commit().await?;
        Ok(id)
    }
}
```

### Idempotent command handling

```rust
use ddd_application::idempotency::{IdempotentCommand, IdempotentCommandHandler};

impl IdempotentCommand for CreateOrder {
    fn idempotency_key(&self) -> &str {
        &self.request_id
    }
}

// Wrap the handler — duplicate calls return the cached result
let handler = IdempotentCommandHandler::new(inner_handler, idempotency_store, ttl);
```

### Saga orchestration

```rust
use ddd_application::saga::{DefaultSagaOrchestrator, SagaDefinitionRegistry};

// Register and start
let mut registry = SagaDefinitionRegistry::new();
registry.register(definition);

let orchestrator = DefaultSagaOrchestrator::new(saga_repo, outbox_repo, Arc::new(registry));

// Start saga — publishes first step's action event via outbox
orchestrator.start("create-order-saga", serde_json::to_value(payload)?).await?;
```

## Rules

- **Zero framework dependencies** (no SeaORM, tonic, axum).
- Unit of Work factory ensures transactional integrity across multiple repositories.
- `Mediator` provides decoupling between API/BFF and the business logic.

