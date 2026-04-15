# ddd-infrastructure

Concrete adapters for the DDD stack: SeaORM repositories (outbox, inbox, dead-letter, idempotency, saga), NATS messaging, OpenTelemetry + Prometheus telemetry, and the outbox relay loop.

## What's inside

| Module | Contents |
|---|---|
| `db/connection` | `create_pool`, `create_pool_from_env` — SeaORM `DatabaseConnection` setup |
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

## Features

- `postgres` (default) — SeaORM + sqlx Postgres adapters.
- `nats` (default) — async-nats publisher/subscriber.
- `telemetry` (default) — OpenTelemetry OTLP + Prometheus.
- `full` — all of the above.

## Examples

### Database connection setup

```rust
use ddd_infrastructure::db::{create_pool, create_pool_from_env};

// From explicit URL
let db = create_pool("postgres://user:pass@localhost:5432/mydb").await?;

// From DATABASE_URL env var
let db = create_pool_from_env().await?;
```

### Wiring all repositories (AppDeps)

```rust
use ddd_infrastructure::db::*;

let db = create_pool_from_env().await?;

let deps = AppDeps {
    db: db.clone(),
    outbox: Arc::new(SeaOrmOutboxRepository::new(db.clone())),
    inbox: Arc::new(SeaOrmInboxRepository::new(db.clone())),
    dead_letter: Arc::new(SeaOrmDeadLetterRepository::new(db.clone())),
    idempotency: Arc::new(DbIdempotencyStore::new(db.clone())),
    saga: Arc::new(SeaOrmSagaInstanceRepository::new(db.clone())),
    uow: Arc::new(SeaOrmUnitOfWorkFactory::new(db.clone())),
};

let mediator = Mediator::from_inventory(&deps);
```

### Outbox Relay

Processes unpublished messages from the database and sends them to NATS.

```rust
use ddd_shared_kernel::{OutboxRelay, NullDeadLetterAlert};

let relay = OutboxRelay::new(
    Arc::new(SeaOrmOutboxRepository::new(db.clone())),
    Arc::new(NatsPublisher::new(nats.clone())),
    Arc::new(SeaOrmDeadLetterRepository::new(db.clone())),
    Arc::new(NullDeadLetterAlert),
    10,    // batch size
    5000,  // poll interval ms
    5      // max attempts
);

tokio::spawn(async move {
    relay.run().await;
});
```

### NATS subscriber with inbox deduplication

```rust
use ddd_shared_kernel::inbox::InboxProcessor;

let processor = InboxProcessor::new(
    Arc::new(SeaOrmInboxRepository::new(db.clone())),
    vec![Arc::new(MyHandler)],
    Arc::new(SeaOrmDeadLetterRepository::new(db.clone())),
    Arc::new(NullDeadLetterAlert),
    10,    // batch size
    5000,  // poll interval ms
    5      // max attempts
);

tokio::spawn(async move {
    processor.run().await;
});
```

// Subscribe to a NATS subject (saves to inbox)
let subscriber = NatsSubscriber::new(
    nats.clone(),
    SeaOrmInboxRepository::new(db.clone()),
    "orders.>".to_owned()
);

tokio::spawn(async move {
    let _ = subscriber.start().await;
});
```

### Running migrations

```rust
use ddd_infrastructure::db::migration_runner::{run_migrations, run_migrations_from_path};

// Auto-discover migrations
run_migrations(&db).await?;

// From a specific directory
run_migrations_from_path(&db, "./migrations").await?;
```

### Telemetry initialization

```rust
use ddd_infrastructure::telemetry;

// Initialize OpenTelemetry tracing + Prometheus metrics + JSON logging
telemetry::init_tracing("my-service")?;
telemetry::init_metrics()?;
telemetry::init_logging()?;
```

## Outbox flow

1. Command handler persists aggregate + appends `OutboxMessage` in one transaction.
2. `OutboxRelay` (background task) polls unprocessed rows, publishes to NATS, marks processed.
3. Failed messages (after `max_attempts`) are moved to dead-letter store; `DeadLetterAlert` notifies.
4. Consumers deduplicate via the inbox.

## Inbox flow

1. NATS subscriber receives message, checks `inbox_messages` by message id.
2. If new, dispatches to the application handler inside a transaction that also inserts the inbox row (exactly-once at the consumer).
3. On retry the row already exists → skip.
4. Failed messages (after `max_attempts`) are moved to dead-letter store.

## Rules

- Do not leak SeaORM or NATS types into signatures that `ddd-domain` or `ddd-application` see. Implement the ports defined there.
- Configuration (connection strings, NATS URL, OTLP endpoint) comes from env; keep defaults in the composition root, not here.
- Each new persistence concern needs a SeaORM entity model in `db/models/` and a repository in `db/`.
