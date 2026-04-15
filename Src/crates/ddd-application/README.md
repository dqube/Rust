# ddd-application

Application-layer building blocks: CQRS dispatch, unit-of-work, validation, use cases, ports (clock, id generator, event publisher), domain-event handler registry, idempotency decorator, saga orchestrator, and the **mediator** that fans out commands / queries / domain events.

## What's inside

| Module | Contents |
|---|---|
| `cqrs` | `Command`, `Query`, `CommandHandler`, `QueryHandler`, `CommandBus`, `QueryBus` |
| `mediator` | `Mediator` facade (`send` / `query` / `publish`) + inventory-based registration |
| `event_handling` | `DomainEventHandler`, `EventHandlerRegistry`, `BoxedDomainEvent` |
| `unit_of_work` | `UnitOfWork`, `UnitOfWorkFactory` |
| `use_case` | `UseCase`, `ValidatedUseCase` |
| `validation` | `Validator`, `ValidatorChain`, `FluentValidator` |
| `validator_registry` | `ValidatorRegistry`, `ErasedValidator`, `ValidatorRegistration` |
| `ports` | `Clock`, `IdGenerator`, `EventPublisher` + default impls (`SystemClock`, `UuidV7Generator`, `NullEventPublisher`) |
| `pagination` | Re-export of `Page`, `PageRequest` + `page_request_from_params` helper |
| `idempotency` | `IdempotentCommand` trait, `IdempotentCommandHandler` decorator |
| `saga` | `DefaultSagaOrchestrator` (state machine), `SagaDefinitionRegistry` |
| `macros` | `register_command_handler!`, `register_query_handler!`, `register_event_handler!` |

## The Mediator

`Mediator` is the single dispatch entry point:

```rust
mediator.send(CreateOrder { ... }).await?;        // command, 1:1 handler
mediator.query(GetOrder { id }).await?;           // query, 1:1 handler
mediator.publish(OrderPlaced { ... }, agg_id, "Order", 1).await?;  // events, 1:N
```

Dispatch is backed by `rustc_hash::FxHashMap<TypeId, Arc<dyn Handler>>` — one lookup + one `Arc` clone per call (~15–30 ns).

### Self-registration

Handlers register themselves at link time through the `inventory` crate:

```rust
register_command_handler!(CreateOrder, AppDeps, |d: &AppDeps| {
    CreateOrderHandler::new(d.repo.clone(), d.outbox.clone())
});

register_query_handler!(GetOrder, AppDeps, |d: &AppDeps| {
    GetOrderHandler::new(d.read_db.clone())
});

register_event_handler!(OrderPlaced, AppDeps, |d: &AppDeps| {
    OrderPlacedProjector::new(d.read_db.clone())
});
```

Then at startup:

```rust
let mediator = Mediator::from_inventory(&deps);
```

No central wiring file. `AppDeps` is defined per service.

For tests or explicit wiring, use the builder:

```rust
let mediator = Mediator::builder()
    .command::<CreateOrder, _>(handler)
    .query::<GetOrder, _>(handler)
    .build();
```

## Examples

### Full command handler with outbox

```rust
use ddd_application::{Command, CommandHandler};
use ddd_shared_kernel::{AppResult, OutboxMessage, OutboxRepository};

pub struct CreateOrder { pub sku: String, pub qty: u32 }
impl Command for CreateOrder { type Response = Uuid; }

pub struct CreateOrderHandler {
    repo: Arc<dyn OrderRepository>,
    outbox: Arc<dyn OutboxRepository>,
    uow: Arc<dyn UnitOfWorkFactory>,
}

#[async_trait]
impl CommandHandler<CreateOrder> for CreateOrderHandler {
    async fn handle(&self, cmd: CreateOrder) -> AppResult<Uuid> {
        let order = Order::place(OrderId::new(), vec![LineItem::new(cmd.sku, cmd.qty)]);
        let id = order.id();

        let mut uow = self.uow.begin().await?;
        self.repo.save(&order).await?;

        // Integration event goes through outbox (same transaction)
        let msg = OutboxMessage::new("order.placed", &OrderPlacedIntegration {
            order_id: id.into(),
        })?;
        self.outbox.store(msg).await?;

        uow.commit().await?;
        Ok(id.into())
    }
}

register_command_handler!(CreateOrder, AppDeps, |d: &AppDeps| {
    CreateOrderHandler::new(d.order_repo.clone(), d.outbox.clone(), d.uow.clone())
});
```

### Query handler

```rust
pub struct GetOrder { pub id: Uuid }
impl Query for GetOrder { type Response = OrderDto; }

pub struct GetOrderHandler { read_db: Arc<dyn OrderReadModel> }

#[async_trait]
impl QueryHandler<GetOrder> for GetOrderHandler {
    async fn handle(&self, q: GetOrder) -> AppResult<OrderDto> {
        self.read_db.find_by_id(q.id).await?
            .ok_or(AppError::not_found("Order", q.id.to_string()))
    }
}

register_query_handler!(GetOrder, AppDeps, |d: &AppDeps| {
    GetOrderHandler::new(d.read_db.clone())
});
```

### Idempotent command handling

```rust
use ddd_application::idempotency::{IdempotentCommand, IdempotentCommandHandler};

impl IdempotentCommand for CreateOrder {
    fn idempotency_key(&self) -> String {
        format!("create-order:{}", self.request_id)
    }
}

// Wrap the handler — duplicate calls return the cached result
let handler = IdempotentCommandHandler::new(inner_handler, idempotency_store);
```

### Saga orchestration

```rust
use ddd_application::saga::{DefaultSagaOrchestrator, SagaDefinitionRegistry};
use ddd_shared_kernel::saga::{SagaDefinition, SagaStepDefinition};

// Define the saga
let definition = SagaDefinition::new("create-order-saga", vec![
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

// Register and start
let mut registry = SagaDefinitionRegistry::new();
registry.register(definition);

let orchestrator = DefaultSagaOrchestrator::new(saga_repo, outbox_repo);

// Start saga — publishes first step's action event via outbox
let instance = orchestrator.start("create-order-saga", correlation_id, payload).await?;

// On step success — advances to next step or completes
orchestrator.handle_step_success(&instance.id, 0).await?;

// On step failure — triggers compensation chain
orchestrator.handle_step_failure(&instance.id, 1, "Payment declined").await?;
```

### Validator registry

```rust
use ddd_application::{ValidatorRegistry, FluentValidator};

let registry = ValidatorRegistry::from_inventory(&deps);
registry.validate(&create_order_cmd).await?;
```

## Integration events

`mediator.publish` is **in-process only**. Integration events flow through the **outbox**: inside a command handler, persist the aggregate + append `OutboxMessage` in the same `UnitOfWork` transaction. The relay in `ddd-infrastructure` publishes to NATS.

## Features

- `tracing` — instrument handlers with `#[tracing::instrument]`.
- `validation` — enables `validator` for richer validation integration.
