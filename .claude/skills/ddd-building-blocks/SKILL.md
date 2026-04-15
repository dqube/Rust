---
name: ddd-building-blocks
description: Guidance for working inside this repo's layered DDD crates (shared-kernel, domain, application, infrastructure, api). Use when adding aggregates, commands, queries, event handlers, repositories, or API adapters — or when reviewing changes for layering violations.
---

# DDD Building Blocks

This repo is a set of reusable building blocks for Rust microservices built on DDD + CQRS + outbox/inbox. Read `.claude/rules/ddd-architecture.md` first — it encodes the invariants. This skill is a how-to for the common tasks.

## Mental model

- `ddd-shared-kernel` is the zero-dep base (errors, ids, pagination, event ports, outbox/inbox ports, validation).
- `ddd-domain` holds pure domain logic: aggregates, value objects, repository traits.
- `ddd-application` orchestrates use cases through commands / queries dispatched by `Mediator`.
- `ddd-infrastructure` provides concrete adapters (SeaORM, NATS, OpenTelemetry).
- `ddd-api` provides gRPC + REST surface with interceptors, middleware, problem-details error mapping.
- `ddd-bff` is a **library** providing reusable BFF building blocks (gRPC client pool, HTTP proxy, observability, OpenAPI/Scalar). Service binaries like `admin-bff` consume it via the `axum-response` feature.

The mediator dispatches commands/queries in ~15–30 ns via `FxHashMap<TypeId, Arc<dyn Handler>>`. Handlers self-register at link time using the `inventory` crate — **there is no central wiring file.**

## Recipes

### Adding a command

1. Define the command struct and impl `Command` (use `impl_command!` for brevity):
   ```rust
   pub struct CreateOrder { pub sku: String, pub qty: u32 }
   impl_command!(CreateOrder, uuid::Uuid);
   ```
2. Write the handler, implementing `CommandHandler<CreateOrder>`. Load aggregate, call domain method, persist, append outbox in the same UoW transaction.
3. Register next to the handler:
   ```rust
   register_command_handler!(CreateOrder, AppDeps, |d: &AppDeps| {
       CreateOrderHandler::new(d.order_repo.clone(), d.outbox.clone(), d.uow.clone())
   });
   ```
4. Never register from `main.rs`. The macro is the only wiring.

### Adding a query

Identical shape, but `Query` / `QueryHandler` / `register_query_handler!`. Queries must not mutate.

### Adding a domain event handler

`register_event_handler!(OrderPlaced, AppDeps, |d| OrderPlacedProjector::new(d.read_db.clone()));`

Events fan out to every registered handler in registration order; first error short-circuits.

### Adding an integration event

1. Implement `IntegrationEvent` for the struct in `ddd-shared-kernel` convention.
2. In the command handler, after state change, append to the outbox:
   ```rust
   self.outbox.append(OutboxMessage::from_integration_event(&evt)?).await?;
   ```
3. The relay (in `ddd-infrastructure`) polls and publishes to NATS. Do not call NATS from the handler directly.

### Adding an aggregate

1. Create the file in `ddd-domain/src/` with the aggregate struct and its domain methods.
2. Implement `AggregateRoot`. Use `record_event!` inside domain methods to raise events.
3. Add a repository trait (port) next to the aggregate, not in infrastructure.
4. Implement the port with SeaORM in `ddd-infrastructure/src/db/`.

### Adding a BFF endpoint (service binary)

Service binaries that consume `ddd-bff` (like `admin-bff`) should:
1. Add an axum handler that calls `pool.channel("service")` → tonic client → `grpc_status_to_app_error` → `app_error_to_problem`.
2. Add an `ApiRoute` entry to the service's `API_ROUTES` constant.
3. Register the axum route. Do not put business logic in the BFF handler; delegate entirely to the downstream gRPC service.
4. Do **not** add `ddd-domain` or `ddd-application` deps to the BFF crate — it depends only on `ddd-shared-kernel`.

### Adding a REST endpoint

Handlers in `ddd-api` convert HTTP → command/query, call `mediator.send(...)` / `mediator.query(...)`, and map `AppError` to `ProblemDetail` via the `IntoResponse` impl. Do not put business logic in HTTP handlers.

### Adding a gRPC service

Same pattern: proto → command via `FromProto`, dispatch through mediator, map `AppError` → `tonic::Status` via `GrpcErrorExt`. Interceptors (auth, tracing, redaction) live in `ddd-api/src/grpc/interceptor.rs`.

## Checklist before done

- [ ] Layering rules respected (check imports).
- [ ] Handler registered via `register_*_handler!` (not `main.rs` wiring).
- [ ] Integration events go through outbox, not `publish`.
- [ ] `cargo test` green in affected crate.
- [ ] `cargo clippy --all-features -- -D warnings` clean.

## Reference docs

- `ddd-api.md` — full spec for the API crate.
- `implementationplan.md` — architectural overview + BFF + observability.
- `mediator.md` — migration plan and self-registration rationale.
