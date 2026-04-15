---
name: ddd-application
description: Guidance for the ddd-application crate — CQRS dispatch, Mediator, unit-of-work, validation, idempotency, saga orchestrator, and use-case primitives. Use when adding commands, queries, handlers, or orchestration logic.
---

# ddd-application

Application-layer orchestration. Dispatches commands/queries through `Mediator`, manages transactions via `UnitOfWork`, provides validation, idempotency, and saga coordination. Depends only on `ddd-shared-kernel`.

## Modules

| Module | Key types |
|--------|-----------|
| `cqrs` | `Command`, `Query`, `CommandHandler`, `QueryHandler`, `CommandBus`, `QueryBus` |
| `mediator` | `Mediator`, `MediatorBuilder`, `HandlerRegistration`, `MediatorRegistry` |
| `event_handling` | `DomainEventHandler`, `EventHandlerRegistry`, `BoxedDomainEvent` |
| `unit_of_work` | `UnitOfWork`, `UnitOfWorkFactory` |
| `use_case` | `UseCase`, `ValidatedUseCase` |
| `validation` | `Validator`, `ValidatorChain`, `FluentValidator` |
| `validator_registry` | `ValidatorRegistry`, `ErasedValidator`, `ValidatorRegistration` |
| `ports` | `Clock`, `IdGenerator`, `EventPublisher`, `SystemClock`, `UuidV7Generator`, `NullEventPublisher` |
| `pagination` | `page_request_from_params`, re-exports `Page`, `PageRequest` |
| `idempotency` | `IdempotentCommand` trait, `IdempotentCommandHandler` decorator |
| `saga` | `DefaultSagaOrchestrator` (state machine), `SagaDefinitionRegistry` |
| `macros` | `register_command_handler!`, `register_query_handler!`, `register_event_handler!` |

## Feature flags

- `tracing` — instrument handlers with `#[tracing::instrument]`.
- `validation` — enables `validator` for richer validation integration.

## Recipes

### Adding a command
```rust
pub struct CreateOrder { pub sku: String, pub qty: u32 }
impl Command for CreateOrder { type Result = Uuid; }

pub struct CreateOrderHandler {
    repo: Arc<dyn OrderRepository>,
    outbox: Arc<dyn OutboxRepository>,
    uow: Arc<dyn UnitOfWorkFactory>,
}

#[async_trait]
impl CommandHandler<CreateOrder> for CreateOrderHandler {
    async fn handle(&self, cmd: CreateOrder) -> AppResult<Uuid> {
        let order = Order::place(OrderId::new(), vec![/* items */]);
        let id = order.id();
        let mut uow = self.uow.begin().await?;
        self.repo.save(&order).await?;
        let msg = OutboxMessage::new("order.placed", &OrderPlacedIntegration { order_id: id.into() })?;
        self.outbox.store(msg).await?;
        uow.commit().await?;
        Ok(id.into())
    }
}

register_command_handler!(CreateOrder, AppDeps, |d: &AppDeps| {
    CreateOrderHandler::new(d.order_repo.clone(), d.outbox.clone(), d.uow.clone())
});
```

### Adding a query
```rust
pub struct GetOrder { pub id: Uuid }
impl Query for GetOrder { type Result = OrderDto; }

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

### Adding a domain event handler
```rust
register_event_handler!(OrderPlaced, AppDeps, |d: &AppDeps| {
    OrderPlacedProjector::new(d.read_db.clone())
});
```
Events fan out to all registered handlers in order; first error short-circuits.

### Using the Mediator
```rust
// Build from inventory (auto-discovered handlers)
let mediator = Mediator::from_inventory(&deps);

// Or build manually for tests
let mediator = Mediator::builder()
    .command::<CreateOrder, _>(handler)
    .query::<GetOrder, _>(query_handler)
    .build();

// Dispatch
let id = mediator.send(CreateOrder { sku: "ABC".into(), qty: 5 }).await?;
let order = mediator.query(GetOrder { id }).await?;
mediator.publish(OrderPlaced { order_id: id }, agg_id, "Order", 1).await?;
```

### Idempotent command handling
```rust
// Wrap any command handler with idempotency
let handler = IdempotentCommandHandler::new(
    inner_handler,
    idempotency_store,  // Arc<dyn IdempotencyStore>
);
// Requires the command to implement IdempotentCommand
impl IdempotentCommand for CreateOrder {
    fn idempotency_key(&self) -> String {
        format!("create-order:{}", self.request_id)
    }
}
```

### Saga orchestration
```rust
// Define saga steps
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

// Register and orchestrate
let mut registry = SagaDefinitionRegistry::new();
registry.register(definition);

let orchestrator = DefaultSagaOrchestrator::new(
    saga_repo,   // Arc<dyn SagaInstanceRepository>
    outbox_repo, // Arc<dyn OutboxRepository>
);
let instance = orchestrator.start("create-order-saga", correlation_id, payload).await?;
```

## Rules

- No framework types (no SeaORM, tonic, axum, async-nats).
- Handlers self-register via macros — no central wiring file.
- Integration events go through the outbox, never `mediator.publish`.
- `mediator.publish` is in-process only (domain event fan-out).
