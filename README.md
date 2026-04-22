# DDD Building Blocks for Rust

Production-ready Domain-Driven Design building blocks for Rust microservices, implementing Clean Architecture with CQRS, event-driven communication, and a Backend for Frontend gateway.

## Architecture

```
                                    ┌──────────────────────┐
                                    │   ddd-shared-kernel   │  zero-dependency base
                                    │  AppError, TypedId,   │  types, ports, events,
                                    │  outbox/inbox, saga,  │  validation primitives
                                    │  domain events, DLQ   │
                                    └──────────┬───────────┘
                                               │
                              ┌────────────────┼────────────────┐
                              │                │                │
                    ┌─────────▼───────┐  ┌─────▼──────────┐    │
                    │   ddd-domain    │  │ ddd-application │    │
                    │  aggregates,    │  │  CQRS, Mediator │    │
                    │  entities,      │  │  UnitOfWork,    │    │
                    │  repositories,  │  │  validation,    │    │
                    │  specs, policies│  │  saga, idempot. │    │
                    └─────────────────┘  └────────┬───────┘    │
                                                  │            │
                              ┌───────────────────┼────────────┤
                              │                   │            │
                    ┌─────────▼───────┐  ┌────────▼───────┐ ┌──▼──────────┐
                    │ ddd-infrastructure│ │    ddd-api     │ │   ddd-bff   │
                    │  SeaORM repos,   │ │  gRPC + REST   │ │  REST GW,   │
                    │  NATS messaging, │ │  error mapping,│ │  gRPC pool, │
                    │  telemetry       │ │  health, OpenAPI│ │  proxy, SSE │
                    └─────────────────┘  └────────────────┘ └──────┬──────┘
                                                                    │
                                                          ┌─────────▼──────────┐
                                                          │     admin-bff       │
                                                          │  REST gateway for   │
                                                          │  admin console      │
                                                          └────────────────────┘
```

**Dependency rule**: arrows point inward only. Inner layers never depend on outer layers.
`ddd-bff` and `admin-bff` depend only on `ddd-shared-kernel` (no domain/application layer dependency).

## Crates

| Crate | Type | Depends on | Purpose |
|-------|------|------------|---------|
| [`ddd-shared-kernel`](Src/crates/ddd-shared-kernel/) | lib | — | Foundation types: `AppError`, `TypedId`, `Page`, outbox/inbox, DLQ, idempotency, saga, domain/integration events, validation |
| [`ddd-domain`](Src/crates/ddd-domain/) | lib | shared-kernel | Pure domain: aggregates, entities, value objects, repository ports, specifications, policies, domain services |
| [`ddd-application`](Src/crates/ddd-application/) | lib | shared-kernel | CQRS dispatch, `Mediator`, `UnitOfWork`, validation, `IdempotentCommandHandler`, saga orchestrator |
| [`ddd-infrastructure`](Src/crates/ddd-infrastructure/) | lib | shared-kernel, application | SeaORM repositories, NATS messaging, OpenTelemetry + Prometheus telemetry |
| [`ddd-api`](Src/crates/ddd-api/) | lib | shared-kernel, application | gRPC (tonic) + REST (axum) building blocks: interceptors, error mapping, health probes, graceful shutdown, OpenAPI |
| [`ddd-bff`](Src/crates/ddd-bff/) | lib | shared-kernel | BFF library: gRPC client pool, HTTP proxy, axum observability, Prometheus metrics, OpenAPI catalogue + merge, graceful shutdown |

## Services

| Service | Type | Depends on | Purpose |
|---------|------|------------|---------|
| [`admin-bff`](Src/Services/admin-bff/) | bin | ddd-bff, ddd-shared-kernel | REST gateway for admin console — fronts product-service (gRPC) and order-service (gRPC) |
| [`order-service`](Src/Services/order-service/) | bin | ddd-api, ddd-application, ddd-infrastructure | Order management with gRPC and REST APIs |
| [`product-service`](Src/Services/product-service/) | bin | ddd-api, ddd-application, ddd-infrastructure | Product management with gRPC and REST APIs, presigned image upload |

Each crate is independently buildable — there is **no root workspace**. Path dependencies are relative (`../ddd-shared-kernel`).

## Key Features

### CQRS + Mediator

The `Mediator` in `ddd-application` provides single-point dispatch for commands, queries, and domain events. Handlers self-register at link time via `inventory`:

```rust
register_command_handler!(CreateOrder, AppDeps, |deps| {
    CreateOrderHandler::new(deps.outbox.clone(), deps.uow.clone())
});

register_query_handler!(GetOrder, AppDeps, |deps| {
    GetOrderHandler::new(deps.db.clone())
});

register_event_handler!(OrderPlaced, AppDeps, |deps| {
    OrderPlacedHandler::new(deps.notifier.clone())
});

let mediator = Mediator::from_inventory(&app_deps);
let order_id = mediator.send(CreateOrder { sku: "WIDGET-42".into() }).await?;
```

### Outbox / Inbox Pattern

Integration events go through the **transactional outbox**, never directly published:

```rust
// Inside a command handler — same UnitOfWork transaction
let mut uow = factory.begin().await?;
order_repo.save(&order, &uow).await?;
outbox_repo.store(OutboxMessage::new("order.placed", &event)?).await?;
uow.commit().await?;
// OutboxRelay polls and publishes to NATS asynchronously
```

### Idempotency

```rust
// Decorator wraps any CommandHandler with idempotency protection:
let handler = IdempotentCommandHandler::new(inner_handler, idempotency_store, ttl);

// REST: extract from Idempotency-Key header (ddd-api)
async fn create_order(IdempotencyKey(key): IdempotencyKey, ...) -> Result<...> { ... }

// gRPC: extract from metadata (ddd-api)
let key = extract_idempotency_key(request.metadata())?;
```

### Saga Orchestrator

Long-running multi-step workflows with automatic compensation:

```rust
let saga_def = SagaDefinition {
    saga_type: "create-order".into(),
    steps: vec![
        SagaStepDefinition {
            name: "reserve-inventory".into(),
            action_event_type: "inventory.reserve".into(),
            compensation_event_type: "inventory.release".into(),
            ..
        },
    ],
};

let mut registry = SagaDefinitionRegistry::new();
registry.register(saga_def);

let orchestrator = DefaultSagaOrchestrator::new(saga_repo, outbox_repo, Arc::new(registry));
orchestrator.start("create-order", serde_json::to_value(payload)?).await?;
```

### Validation

Fluent validation API in `ddd-shared-kernel` with macros for brevity:

```rust
use ddd_shared_kernel::{validate, validate_all};

let result = validate_all!(
    validate!(email, "email").not_empty().email().into(),
    validate!(&age, "age").positive().in_range(18, 120).into()
);
```

### Health & Readiness Probes

```rust
use ddd_api::rest::{HealthCheckRegistry, health_router};

let registry = Arc::new(HealthCheckRegistry::new());
registry.register(DbHealthCheck::new(db.clone()));

let app = Router::new()
    .merge(health_router(registry));
// GET /health — liveness (always 200)
// GET /ready  — readiness (all checks must pass)
```

### Global Exception Handling

- **REST**: `catch_panic_layer` + `fallback_handler` + RFC 9457 ProblemDetail with `urn:problem-type:*` URIs
- **gRPC**: `error_mapping_interceptor` attaching `problem-details-bin` and `bad-request-bin` metadata

### Graceful Shutdown

```rust
// axum via ddd-bff
axum::serve(listener, app)
    .with_graceful_shutdown(ddd_bff::edge::shutdown::wait_for_shutdown_signal())
    .await?;

// REST Server via ddd-api
RestServer::new()
    .with_router(app)
    .run()
    .await?;
```

### Backend for Frontend (BFF)

`ddd-bff` is a **library** providing reusable BFF building blocks. `admin-bff` is an example consumer:

```
Browser  ──► admin-bff (axum, :3001)
                 ├─ /admin/products/*  ──gRPC──► product-service (:50052)
                 ├─ /admin/orders/*    ──gRPC──► order-service   (:50051)
                 ├─ /admin/orders/batch  fan-out → order-service
                 └─ /admin/catalog/summary  fan-out → product-service
```

Key ddd-bff building blocks:

- **`GrpcClientPool`** — pooled tonic channels with timeout + concurrency limiting
- **`proxy_handler`** — generic HTTP reverse proxy (`ProxyState`, strips prefix, drops hop-by-hop headers)
- **`observability_middleware`** — request logging + Prometheus metrics (`bff_http_requests_total`, etc.)
- **`openapi::inject_routes`** — declarative endpoint catalogue → OpenAPI path items
- **`openapi::openapi_router`** — Scalar UI at `/scalar` + JSON spec at `/api-docs/openapi.json`
- **`openapi::merged_openapi`** — fetch + merge a downstream service's OpenAPI spec
- **`metrics_handler`** — axum handler for Prometheus text scrape
- **`wait_for_shutdown_signal`** — SIGTERM/SIGINT future for `with_graceful_shutdown`

---

## Project Structure

```
Src/
├── crates/
│   ├── ddd-shared-kernel/     # Foundation: AppError, TypedId, outbox/inbox, DLQ, saga, validation
│   ├── ddd-domain/            # Pure domain: aggregates, specs, policies, repository ports
│   ├── ddd-application/       # CQRS, Mediator, UnitOfWork, idempotency, saga orchestrator
│   ├── ddd-infrastructure/    # SeaORM repos, NATS messaging, OpenTelemetry telemetry
│   ├── ddd-api/               # gRPC + REST adapters, ProblemDetail, health, OpenAPI
│   └── ddd-bff/               # BFF library: gRPC pool, proxy, observability, OpenAPI, metrics
└── service/
    ├── admin-bff/             # REST gateway — admin console (product + order)
    ├── order-service/         # Order management — gRPC + REST
    └── product-service/       # Product management — gRPC + REST, presigned upload
```

---

## Quick Start

```bash
# Build a specific crate
cargo check --manifest-path Src/crates/ddd-api/Cargo.toml --all-features

# Build all crates
for dir in ddd-shared-kernel ddd-domain ddd-application ddd-infrastructure ddd-api ddd-bff; do
  cargo check --manifest-path Src/crates/$dir/Cargo.toml --all-features
done

# Build all services
cargo build --manifest-path Src/Services/order-service/Cargo.toml
cargo build --manifest-path Src/Services/product-service/Cargo.toml
cargo build --manifest-path Src/Services/admin-bff/Cargo.toml

# Run all tests
for dir in ddd-shared-kernel ddd-domain ddd-application ddd-infrastructure ddd-api ddd-bff; do
  cargo test --manifest-path Src/crates/$dir/Cargo.toml
done

# Clippy (strict)
for dir in ddd-shared-kernel ddd-domain ddd-application ddd-infrastructure ddd-api ddd-bff; do
  cargo clippy --manifest-path Src/crates/$dir/Cargo.toml --all-targets --all-features -- -D warnings
done
```

---

## Feature Flags

| Crate | Features | Default |
|-------|----------|---------|
| `ddd-shared-kernel` | `validation`, `grpc`, `jwt` | — |
| `ddd-domain` | `tracing` | — |
| `ddd-application` | `tracing`, `validation` | — |
| `ddd-infrastructure` | `postgres`, `nats`, `telemetry`, `full` | `postgres`, `nats`, `telemetry` |
| `ddd-api` | `grpc`, `rest`, `openapi`, `telemetry`, `full` | `grpc`, `rest`, `openapi` |
| `ddd-bff` | `axum-response` (enables proxy, openapi router/merge, axum observability) | — |

---

## Design Principles

1. **Strict layering** — Dependencies flow inward only. Domain and application layers have zero framework dependencies.
2. **Ports and adapters** — Repository traits in `ddd-domain`, concrete implementations in `ddd-infrastructure`.
3. **Transactional outbox** — Integration events are never published directly; they go through the outbox for reliable delivery.
4. **Self-registering handlers** — `inventory` crate discovers handlers at link time; no central wiring files.
5. **RFC 9457 everywhere** — All error responses follow Problem Details format with typed URIs and field violations.
6. **BFF as a reusable library** — `ddd-bff` provides generic building blocks; services supply their own wiring and schema docs.
7. **Observable by default** — Structured logging, distributed tracing (OpenTelemetry), and Prometheus metrics built in.

---

## Design References

| Document | Contents |
|----------|----------|
| [`CLAUDE.md`](CLAUDE.md) | AI assistant guidance — commands, patterns, design decisions |
| [`GRAPH_REPORT.md`](graphify-out/GRAPH_REPORT.md) | Architectural knowledge graph summary |

## License

MIT
