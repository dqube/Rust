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
- Do not turn `ddd-bff` into a binary — it is a library; create a named service binary (e.g. `admin-bff`) under `Src/service/` that consumes it.

## Verification

Before declaring a task complete, run from the affected crate directory:

```
cargo check --all-features
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```
