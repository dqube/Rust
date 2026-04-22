# ddd-infrastructure

Concrete adapters for the DDD stack: SeaORM repositories (outbox, inbox, dead-letter, idempotency, saga), NATS messaging, OpenTelemetry + Prometheus telemetry, and the outbox relay loop.

## What's inside

| Module | Contents |
|---|---|
| `db/connection` | `create_pool`, `create_pool_from_env` |
| `db/base_repository` | `BaseRepository` — shared CRUD helpers |
| `db/unit_of_work` | `SeaOrmUnitOfWork`, `SeaOrmUnitOfWorkFactory` |
| `db/outbox_repository` | `SeaOrmOutboxRepository` |
| `db/inbox_repository` | `SeaOrmInboxRepository` |
| `db/dead_letter_repository` | `SeaOrmDeadLetterRepository` |
| `db/idempotency_store` | `DbIdempotencyStore` |
| `db/saga_repository` | `SeaOrmSagaInstanceRepository` |
| `db/migration_runner` | `run_migrations` |
| `messaging/nats_publisher` | NATS publisher adapter |
| `messaging/nats_subscriber` | NATS subscriber with inbox idempotency |
| `telemetry` | Logging, metrics, and tracing (OTLP) initialization |

## Standalone Examples

For full implementation details, see:
- [`outbox_relay_setup.rs`](examples/outbox_relay_setup.rs) — Wiring a SeaORM outbox with NATS and the relay worker.
- [`telemetry_init.rs`](examples/telemetry_init.rs) — Initializing the full observability stack (Logging, Tracing, Metrics).

## Examples

### Database connection setup

```rust
use ddd_infrastructure::db::{create_pool, create_pool_from_env};

// From DATABASE_URL env var
let db = create_pool_from_env().await?;
```

### Outbox Relay

Processes unpublished messages from the database and sends them to NATS.

```rust
use ddd_shared_kernel::{OutboxRelay, NullDeadLetterAlert};

let relay = OutboxRelay::new(
    outbox_repo,
    publisher,
    dead_letter_repo,
    Arc::new(NullDeadLetterAlert),
    10,    // batch size
    1000,  // poll interval ms
    5      // max attempts
);

tokio::spawn(async move {
    relay.run().await;
});
```

### Telemetry initialization

```rust
use ddd_infrastructure::telemetry::{init_telemetry, shutdown_telemetry};

// Initialize Logging, Tracing (OTLP), and Metrics (Prometheus)
init_telemetry("my-service")?;

// At shutdown:
shutdown_telemetry();
```

## Rules

- Implement the ports defined in `ddd-domain` or `ddd-application`.
- Configuration comes from env; keep defaults in the composition root.
- Each persistence concern needs a SeaORM model in `db/models/`.
- Each new persistence concern needs a repository in `db/`.
