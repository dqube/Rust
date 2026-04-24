# DDD Building Blocks — Architecture Rules

These rules apply to all work in this repository. They encode invariants of the layered DDD architecture; violating them creates cycles or leaks concerns across boundaries.

## Crate layering

Dependency direction is strictly inward:

```
ddd-shared-kernel   (no internal deps)
  ↑
ddd-domain          (depends on: ddd-shared-kernel)
  ↑
ddd-application     (depends on: ddd-shared-kernel; may depend on ddd-domain)
  ↑
ddd-infrastructure  (depends on: ddd-shared-kernel, ddd-application)
ddd-api             (depends on: ddd-shared-kernel, ddd-application)
ddd-bff             (depends on: ddd-shared-kernel)   ← library, BFF building blocks
  ↑
admin-bff           (depends on: ddd-bff, ddd-shared-kernel)   ← binary, REST gateway
```

- `ddd-domain` and `ddd-application` **must not** depend on `ddd-infrastructure`, `ddd-api`, or `ddd-bff`.
- `ddd-infrastructure`, `ddd-api`, and `ddd-bff` **must not** depend on each other.
- `ddd-bff` is a **library crate** — provides reusable BFF building blocks consumed by service binaries like `admin-bff`.
- New crates follow the `ddd-` prefix in both directory and `Cargo.toml` `name`.

## Where things live

| Concept | Crate |
|---|---|
| `AppError`, `AppResult`, `Page`, `PageRequest`, `TypedId` | `ddd-shared-kernel` |
| Domain events, integration events, outbox/inbox **ports** | `ddd-shared-kernel` |
| Dead-letter queue ports (`DeadLetterRepository`, `DeadLetterAlert`) | `ddd-shared-kernel` |
| Idempotency port (`IdempotencyStore`, `IdempotencyRecord`) | `ddd-shared-kernel` |
| Saga types/ports (`SagaDefinition`, `SagaInstance`, `SagaOrchestrator`) | `ddd-shared-kernel` |
| Fluent validation API, `validate!`, `validate_all!` | `ddd-shared-kernel` |
| Aggregates, entities, value objects, repository traits, specifications, policies | `ddd-domain` |
| Commands, queries, handlers, `Mediator`, `UnitOfWork`, use cases | `ddd-application` |
| `IdempotentCommandHandler` decorator | `ddd-application` |
| `DefaultSagaOrchestrator`, `SagaDefinitionRegistry` | `ddd-application` |
| `ValidatorRegistry`, `ValidatorChain`, `FluentValidator` | `ddd-application` |
| SeaORM repositories (outbox, inbox, dead-letter, idempotency, saga) | `ddd-infrastructure` |
| NATS messaging, OpenTelemetry, outbox relay | `ddd-infrastructure` |
| gRPC (tonic) + REST (axum) adapters, interceptors, OpenAPI | `ddd-api` |
| RFC 9457 ProblemDetail, `FieldViolation`, global exception handlers | `ddd-api` |
| Health/readiness probes (`HealthCheck`, `health_router`) | `ddd-api` |
| Graceful shutdown (`RestServer`, `GrpcServer`) | `ddd-api` |
| Idempotency key extractors (REST `IdempotencyKey`, gRPC `extract_idempotency_key`) | `ddd-api` |
| gRPC client pool (`GrpcClientPool`, `ResilientChannel`) | `ddd-bff` |
| Generic HTTP proxy (`ProxyState`, `proxy_handler`) | `ddd-bff` |
| Axum observability middleware (request logging, body redaction, Prometheus metrics) | `ddd-bff` |
| BFF error mapping (`grpc_status_to_app_error`, `app_error_to_problem`) | `ddd-bff` |
| BFF configuration (`BffConfig`, `ServiceUrls`, `ResilienceConfig`, `env_or`) | `ddd-bff` |
| OpenAPI catalogue (`ApiRoute`, `RouteKind`, `inject_routes`), Scalar router (`openapi_router`), spec merge (`merged_openapi`) | `ddd-bff` |
| Graceful shutdown (`wait_for_shutdown_signal`) | `ddd-bff` |
| Prometheus metrics (`BFF_METRICS`, `metrics_handler`) | `ddd-bff` |
| BFF gateway binary (consumes `ddd-bff` in pick-and-mix style) | `admin-bff` (service) |

No framework types in `ddd-domain` or `ddd-application` (no `sea_orm`, `tonic`, `axum`, `async_nats`).

## The Mediator

`ddd-application::Mediator` is the single dispatch entry point:

- `mediator.send(cmd)` — commands (1:1 handler)
- `mediator.query(q)` — queries (1:1 handler)
- `mediator.publish(event, agg_id, agg_type, version)` — domain events (1:N, in-process)

Handlers self-register via `register_command_handler!` / `register_query_handler!` / `register_event_handler!` placed next to the handler. Do not add central wiring files — `inventory` discovers registrations at link time.

**Integration events go through the outbox, not `publish`.** Inside a command handler: persist aggregate + append `OutboxMessage` in the same `UnitOfWork` transaction. The background relay publishes to NATS.

## Dependency injection

- Define one `AppDeps` struct per service (e.g. `Arc<DatabaseConnection>`, `Arc<dyn OutboxRepository>`).
- Build once: `Mediator::from_inventory(&deps)`.
- Handler registration closures extract what they need from `&AppDeps`; handlers own their deps.

## What NOT to do

- Do not introduce a second mediator / parallel bus. Extend `Mediator` if something is missing.
- Do not hand-write `inventory::submit!` — always use the `register_*_handler!` macros.
- Do not `publish` an integration event directly. Use the outbox.
- Do not add a root `Cargo.toml` workspace without asking — crates are intentionally independent.
- Do not call NATS / external services from command handlers — use the outbox for integration events.
- Do not put business logic in REST/gRPC handlers — all orchestration goes through `Mediator`.
- Do not define repository implementations in `ddd-domain` — only ports (traits). Impls go in `ddd-infrastructure`.
- Do not add business logic in `ddd-bff` handlers — BFF delegates to downstream gRPC services only.
- Do not add `ddd-domain` or `ddd-application` as dependencies of `ddd-bff` — it depends only on `ddd-shared-kernel`.
- Do not turn `ddd-bff` into a binary — it is a library; create a named service binary (e.g. `admin-bff`) under `Src/Services/` that consumes it.

## Domain Model Migration Rules (Aggregate / Entity / Value Object)

When migrating a service to the DDD architecture, the domain layer **must** use the macros from `ddd-domain` and `ddd-shared-kernel`. Plain structs are not acceptable for aggregates or entities.

### Aggregate (root — owns its lifecycle and raises domain events)

```rust
// ✅ DO
use ddd_domain::{define_aggregate, impl_aggregate, impl_aggregate_events};
use ddd_shared_kernel::{AppResult, DomainEvent};

define_aggregate!(Store, StoreId, {
    pub name: String,
    pub status: StoreStatus,
    // ... fields
});

impl_aggregate!(Store, StoreId);
impl_aggregate_events!(Store, StoreCreated, StoreUpdated);   // all events this root raises

impl Store {
    pub fn create(name: String, ...) -> AppResult<Self> { ... }  // factory, returns AppResult
    pub fn update_name(&mut self, name: String) -> AppResult<()> { ... }
}

// ❌ DON'T
#[derive(Debug, Clone)]
pub struct Store {
    pub id: StoreId,
    pub name: String,
    ...
}
impl Store {
    pub fn create(...) -> Self { ... }
}
```

### Entity (has identity, owned by an aggregate)

```rust
// ✅ DO
use ddd_domain::define_entity;

define_entity!(StoreSchedule, StoreScheduleId, {
    pub store_id:    StoreId,
    pub day_of_week: u8,
    pub open_time:   Option<String>,
    pub close_time:  Option<String>,
    pub is_closed:   bool,
});

impl StoreSchedule {
    pub fn new(id: StoreScheduleId, store_id: StoreId, ...) -> Self { ... }
}

// ❌ DON'T
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreSchedule {
    pub day_of_week: u8,
    ...
}
```

### Value Object (no identity, compared by value)

```rust
// ✅ DO
use ddd_shared_kernel::define_value_object;   // or implement PartialEq + Clone manually

#[derive(Debug, Clone, PartialEq)]
pub struct Address {
    pub street:      String,
    pub city:        String,
    pub postal_code: String,
    pub country:     String,
}

// Value objects go in the aggregate's field, NOT as a separate entity with an id.
```

### Domain Events (declared alongside the aggregate)

```rust
// ✅ DO — in domain/events.rs
use ddd_shared_kernel::DomainEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreCreated {
    pub store_id: StoreId,
    pub name:     String,
}
impl DomainEvent for StoreCreated {
    fn event_type(&self) -> &'static str { "StoreCreated" }
    fn aggregate_type(&self) -> &'static str { "Store" }
}

// ❌ DON'T raise events from infrastructure or command handlers directly.
// Events are raised by aggregate methods and collected via impl_aggregate_events!.
```

### Repository Port (domain/repositories.rs)

```rust
// ✅ DO — trait only, no SeaORM/sqlx
#[async_trait]
pub trait StoreRepository: Send + Sync {
    async fn find_by_id(&self, id: StoreId) -> AppResult<Option<Store>>;
    async fn save(&self, store: &Store) -> AppResult<()>;
}

// ❌ DON'T put SeaORM models or DB logic here. That belongs in ddd-infrastructure.
```

### File layout expected after migration

```
domain/
  mod.rs
  ids.rs          ← declare_id!(StoreId); declare_id!(StoreScheduleId);
  enums.rs        ← StoreStatus, RegisterStatus
  events.rs       ← StoreCreated, StoreUpdated, ...
  repositories.rs ← trait StoreRepository, trait RegisterRepository
  entities/
    mod.rs
    store.rs      ← define_aggregate! + impl_aggregate! + impl_aggregate_events! + methods
    register.rs   ← define_aggregate! or define_entity! depending on ownership
```

### Migration checklist

- [ ] Every aggregate uses `define_aggregate!` + `impl_aggregate!` + `impl_aggregate_events!`
- [ ] Every child entity uses `define_entity!`
- [ ] Inline struct fields that have no identity become value objects (no id, no macro)
- [ ] `domain/events.rs` exists and all domain events implement `DomainEvent`
- [ ] `domain/repositories.rs` contains only traits — no `sea_orm`, no `sqlx`, no `Arc`
- [ ] `domain/ids.rs` uses `declare_id!` for every strongly-typed id
- [ ] No `Serialize`/`Deserialize` derives on aggregates or entities (only on DTOs and events)
- [ ] Aggregate factory methods return `AppResult<Self>`, not bare `Self`

## Verification

Before declaring a task complete, run from the affected crate directory:

```
cargo check --all-features
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```
