---
name: ddd-infrastructure
description: Guidance for the ddd-infrastructure crate — SeaORM repositories, NATS messaging, OpenTelemetry telemetry, and concrete adapter implementations. Use when adding database persistence, messaging adapters, or observability wiring.
---

# ddd-infrastructure

Concrete adapters implementing the ports defined in `ddd-shared-kernel` and `ddd-domain`. Depends on `ddd-shared-kernel` + `ddd-application`.

## Modules

| Module | Key types |
|--------|-----------|
| `db/connection` | `create_pool`, `create_pool_from_env` (SeaORM `DatabaseConnection`) |
| `db/base_repository` | `BaseRepository` — shared CRUD helpers |
| `db/unit_of_work` | `SeaOrmUnitOfWork`, `SeaOrmUnitOfWorkFactory` |
| `db/outbox_repository` | `SeaOrmOutboxRepository` implements `OutboxRepository` |
| `db/inbox_repository` | `SeaOrmInboxRepository` implements `InboxRepository` |
| `db/dead_letter_repository` | `SeaOrmDeadLetterRepository` implements `DeadLetterRepository` |
| `db/idempotency_store` | `DbIdempotencyStore` implements `IdempotencyStore` |
| `db/saga_repository` | `SeaOrmSagaInstanceRepository` implements `SagaInstanceRepository` |
| `db/migration_runner` | `run_migrations`, `run_migrations_from_path` |
| `db/models/` | SeaORM entity models: `outbox`, `inbox`, `dead_letter`, `idempotency`, `saga` |
| `messaging/nats_publisher` | NATS publisher adapter |
| `messaging/nats_subscriber` | NATS subscriber with inbox idempotency |
| `messaging/event_serializer` | JSON (de)serialization for integration events |
| `telemetry/logging` | Structured JSON logging initialization |
| `telemetry/metrics` | Prometheus metrics setup |
| `telemetry/tracing` | OTLP tracing setup |

## Feature flags

- `postgres` (default) — SeaORM + sqlx Postgres adapters.
- `nats` (default) — async-nats publisher/subscriber.
- `telemetry` (default) — OpenTelemetry OTLP + Prometheus.
- `full` — all of the above.

## Recipes

### Adding a new repository implementation
1. Create a SeaORM entity model in `db/models/<name>.rs`.
2. Create `db/<name>_repository.rs` implementing the port trait from `ddd-shared-kernel` or `ddd-domain`.
3. Add the module to `db/mod.rs` and export the concrete type.

```rust
use ddd_shared_kernel::{AppResult, DeadLetterMessage, DeadLetterRepository};

pub struct SeaOrmDeadLetterRepository {
    db: DatabaseConnection,
}

#[async_trait]
impl DeadLetterRepository for SeaOrmDeadLetterRepository {
    async fn store(&self, message: DeadLetterMessage) -> AppResult<()> {
        let model = dead_letter::ActiveModel::from(message);
        dead_letter::Entity::insert(model).exec(&self.db).await?;
        Ok(())
    }
    // ... other methods
}
```

### Outbox flow
1. Command handler persists aggregate + appends `OutboxMessage` in one transaction.
2. `OutboxRelay` (background task) polls unprocessed rows, publishes to NATS, marks processed.
3. Failed messages (after `max_attempts`) are moved to dead-letter store.
4. Consumers deduplicate via the inbox.

### Inbox flow
1. NATS subscriber receives message, checks `inbox_messages` by message id.
2. If new: dispatches to application handler inside a transaction that also inserts the inbox row (exactly-once).
3. On retry: row exists → skip.
4. Failed messages (after `max_attempts`) are moved to dead-letter store.

### Wiring AppDeps
```rust
let db = create_pool_from_env().await?;
let outbox_repo = Arc::new(SeaOrmOutboxRepository::new(db.clone()));
let inbox_repo = Arc::new(SeaOrmInboxRepository::new(db.clone()));
let dead_letter_repo = Arc::new(SeaOrmDeadLetterRepository::new(db.clone()));
let idempotency_store = Arc::new(DbIdempotencyStore::new(db.clone()));
let saga_repo = Arc::new(SeaOrmSagaInstanceRepository::new(db.clone()));
let uow_factory = Arc::new(SeaOrmUnitOfWorkFactory::new(db.clone()));

let deps = AppDeps {
    db: db.clone(),
    outbox: outbox_repo,
    inbox: inbox_repo,
    dead_letter: dead_letter_repo,
    idempotency: idempotency_store,
    saga: saga_repo,
    uow: uow_factory,
};
let mediator = Mediator::from_inventory(&deps);
```

## Rules

- Do not leak SeaORM or NATS types into signatures visible to `ddd-domain` or `ddd-application`. Implement the ports defined there.
- Configuration (connection strings, NATS URL, OTLP endpoint) comes from env; keep defaults in the composition root, not here.
- Each new model needs a corresponding `db/models/<name>.rs` SeaORM entity.
