# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Layout

This repo implements reusable DDD (Domain-Driven Design) building blocks for Rust microservices. Crates live under `Src/crates/` and are layered bottom-up:

- `ddd-sharedkernel` (package name: `ddd-shared-kernel`) — zero-dependency base: `AppError`/`AppResult`, ids, pagination (`Page`, `PageRequest`), outbox/inbox, dead-letter queue, idempotency store, saga types/ports, validation primitives, domain events, integration events.
- `ddd-domain` — aggregates, entities, repositories (ports), specifications, policies, domain services. Depends only on `ddd-shared-kernel`.
- `ddd-application` — CQRS (`Command`/`Query`), `Mediator` (inventory-based dispatch), use cases, `UnitOfWork`, ports, validation, pagination, event handling, `IdempotentCommandHandler`, `DefaultSagaOrchestrator`, `SagaDefinitionRegistry`. Depends on `ddd-shared-kernel`.
- `ddd-infrastructure` — adapters: SeaORM/sqlx (Postgres) repositories (outbox, inbox, dead-letter, idempotency, saga), NATS messaging (core + JetStream), S3-compatible blob storage (`S3BlobStorage`), Redis cache (`RedisCache`), security (`Pbkdf2Hasher`, `AesGcmCipher`), OpenTelemetry traces + logs + Prometheus telemetry. Depends on `ddd-shared-kernel` + `ddd-application`.
- `ddd-api` — gRPC (tonic) + REST (axum) building blocks: interceptors, middleware, error mapping (`AppError` → `tonic::Status` / RFC 9457 Problem Details with `FieldViolation`), global exception handlers, health/readiness probes, graceful shutdown, idempotency key extractors, pagination DTOs, OpenAPI/Scalar. Depends on `ddd-shared-kernel` + `ddd-application`.
- `ddd-bff` — **library crate** for BFF gateways. gRPC client pool (`GrpcClientPool`, `ResilientChannel`), generic HTTP proxy (`ProxyState`, `proxy_handler`), axum observability middleware, Prometheus metrics (`BFF_METRICS`, `metrics_handler`), declarative OpenAPI catalogue (`ApiRoute`, `inject_routes`), Scalar router (`openapi_router`), downstream spec merge (`merged_openapi`), graceful shutdown (`wait_for_shutdown_signal`), body redaction. Depends only on `ddd-shared-kernel`.

Services live under `Src/service/`:

- `admin-bff` — binary REST gateway for the admin console; consumes `ddd-bff` in pick-and-mix style. Fronts `product-service` (gRPC, port 50052) and `order-service` (gRPC, port 50051).
- `order-service` — order management service with gRPC + REST APIs.
- `product-service` — product management service with gRPC + REST APIs and presigned image upload.

**Dependency rule**: inner layers never depend on outer layers. `domain` and `application` must not reference `infrastructure`, `api`, or `bff`. `ddd-bff` depends only on `ddd-shared-kernel` — no domain or application layer. `admin-bff` depends on `ddd-bff` + `ddd-shared-kernel`.

The repo is a Cargo workspace rooted at `/Cargo.toml`. Members include all six `ddd-*` crates and all three services. `[workspace.package]` carries shared `version`, `edition`, `license`, and `rust-version`, and `[workspace.dependencies]` pins every third-party version once — individual crates declare deps as `{ workspace = true }`. Build with `cargo check -p <crate>` from the repo root; per-crate `Cargo.lock` files are not used and should not be committed.

## Crate Naming Convention

All crates under `Src/crates/` use the `ddd-` prefix in both the directory name and the `name` field of `Cargo.toml`. When adding a new crate or dependency, preserve this prefix.

Note: `ddd-sharedkernel` is the directory name, but the Cargo package is `ddd-shared-kernel` (with hyphen). Use `ddd-shared-kernel` in `[dependencies]`.

## Common Commands

Run from the repo root. The workspace resolves `-p <package>` to the correct crate; prefer that over `--manifest-path`.

```bash
# Check / build
cargo check -p ddd-api --all-features
cargo build -p admin-bff

# Test
cargo test -p ddd-application
cargo test -p ddd-application -- --nocapture

# Lint
cargo clippy -p ddd-bff --all-targets --all-features -- -D warnings
cargo fmt -p ddd-api

# Everything
cargo check --workspace --all-features
```

Feature flags worth knowing:

| Crate | Features |
|-------|----------|
| `ddd-shared-kernel` | `validation`, `grpc`, `jwt`, `config-validation` |
| `ddd-domain`, `ddd-application` | `tracing` (`validation` on application) |
| `ddd-infrastructure` | `postgres`, `nats`, `nats-jetstream`, `storage`, `crypto`, `cache`, `telemetry` (all default), `full` |
| `ddd-api` | `grpc`, `rest`, `openapi` (all default), `telemetry`, `jwt`, `full` |
| `ddd-bff` | `axum-response` (enables `proxy`, `openapi::router`, `openapi::merge`, `middleware::axum_observability`, `prelude`); `jwt` (implies `axum-response`) |

## Key Patterns

### Mediator (CQRS dispatch)

`Mediator` in `ddd-application` dispatches commands, queries, and domain events. Handlers self-register at link time via `inventory`:

```rust
register_command_handler!(CreateOrder, AppDeps, |d: &AppDeps| {
    CreateOrderHandler::new(d.order_repo.clone(), d.outbox.clone(), d.uow.clone())
});
register_query_handler!(GetOrder, AppDeps, |d: &AppDeps| {
    GetOrderHandler::new(d.read_db.clone())
});
register_event_handler!(OrderPlaced, AppDeps, |d: &AppDeps| {
    OrderPlacedProjector::new(d.read_db.clone())
});
```

Build once: `let mediator = Mediator::from_inventory(&deps);`

Do **not** register handlers from `main.rs`. Do **not** hand-write `inventory::submit!`.

### Outbox / Inbox

Integration events go through the outbox, never `mediator.publish`. Inside a command handler, persist aggregate + append `OutboxMessage` in the same `UnitOfWork` transaction. `OutboxRelay` publishes via an `IntegrationEventPublisher` — either `NatsPublisher` (core NATS, fire-and-forget) or `JetStreamPublisher` (durable, at-least-once; feature `nats-jetstream`). See `Src/crates/ddd-infrastructure/examples/outbox_relay_setup.rs` and `outbox_relay_jetstream.rs`. `InboxProcessor` deduplicates at the consumer.

### Blob Storage

`BlobStorage` port in `ddd-shared-kernel` exposes `presigned_put` / `presigned_get` returning `PresignedUrl { url, expires_at }`. The `S3BlobStorage` adapter (`ddd-infrastructure`, feature `storage`) targets AWS S3, MinIO, SeaweedFS — see `S3Config::endpoint` + `force_path_style` for S3-compatible servers. `product-service` uses it to mint product image upload URLs (configured via `S3_*` env vars + `PRODUCT_IMAGE_BUCKET` + `PRODUCT_PRESIGN_TTL_SECS`).

### Cache

`Cache` port (`get_raw` / `set_raw` / `delete`) plus `CacheExt` blanket trait (JSON `get` / `set` / `get_or_set`) in `ddd-shared-kernel`. `RedisCache` adapter (`ddd-infrastructure`, feature `cache`) supports best-effort (`connect`) and strict (`connect_strict`) semantics with prefixed keys. `admin-bff` opts in via `REDIS_URL` and uses it as a read-through cache for `/admin/catalog/summary` (TTL: `CACHE_CATALOG_SUMMARY_TTL_SECS`, default 30 s).

### Security

`Hasher` (sync, password hashing/verification) and `Cipher` (async, authenticated encryption) ports in `ddd-shared-kernel`. Adapters in `ddd-infrastructure` (feature `crypto`): `Pbkdf2Hasher` (PBKDF2-HMAC-SHA256, PHC string format) and `AesGcmCipher` (AES-256-GCM with a 12-byte random nonce prepended to the ciphertext).

### Telemetry

Traces and logs both export via OTLP/gRPC. `init_log_pipeline(service_name)` in `ddd-infrastructure::telemetry::logs` is idempotent and shutdown via `shutdown_logs()`. `OTEL_EXPORTER_OTLP_ENDPOINT` (default `http://localhost:4317`) drives both spans and logs; set `OTEL_LOGS_EXPORTER=none` to disable log export for local dev. The panic hook installed by `install_panic_hook()` flushes both pipelines before re-raising.

### Dead Letter Queue

Failed outbox/inbox messages move to the dead-letter store after `max_attempts`. `DeadLetterAlert` notifies (default: `LogDeadLetterAlert`). Ports in `ddd-shared-kernel`, persistence in `ddd-infrastructure`.

### Idempotency

`IdempotencyStore` port + `IdempotentCommandHandler` decorator (`ddd-application`). `DbIdempotencyStore` (`ddd-infrastructure`). REST `IdempotencyKey` extractor and gRPC `extract_idempotency_key` (`ddd-api`).

### Saga Orchestrator

`SagaDefinition` + `SagaInstance` + `SagaOrchestrator` port (`ddd-shared-kernel`). `DefaultSagaOrchestrator` state machine + `SagaDefinitionRegistry` (`ddd-application`). `SeaOrmSagaInstanceRepository` (`ddd-infrastructure`).

### Health / Readiness Probes

`HealthCheck` trait + `HealthCheckRegistry` + `health_router()` mounting `/health` and `/ready` (`ddd-api/rest`).

### Graceful Shutdown

**ddd-api**: `RestServer` / `GrpcServer` with `with_graceful_shutdown` / `serve_with_shutdown`.

**axum via ddd-bff**: use `ddd_bff::edge::shutdown::wait_for_shutdown_signal()`:
```rust
axum::serve(listener, app)
    .with_graceful_shutdown(ddd_bff::edge::shutdown::wait_for_shutdown_signal())
    .await?;
```

### Global Exception Handling

REST: `catch_panic_layer` + `fallback_handler` + RFC 9457 ProblemDetail with `urn:problem-type:*` URIs and `FieldViolation` arrays. gRPC: `error_mapping_interceptor` attaching `problem-details-bin` and `bad-request-bin` metadata.

### Route Parameters (axum 0.8)

Use `{param}` syntax, not `:param`:
```rust
.route("/products/{id}", get(get_product))
.route("/orders/{id}/items/{item_id}", put(update_item))
```

### BFF (Backend for Frontend)

`ddd-bff` is a **library crate** — not a binary. It provides reusable BFF building blocks:

**axum-response feature** (required for axum-based BFF):

```rust
// Generic HTTP reverse proxy
use ddd_bff::proxy::{ProxyState, proxy_handler};
let proxy = ProxyState::new("http://upstream:8080".into(), "/prefix".into(), Duration::from_secs(5));
Router::new().route("/prefix/{*path}", any(proxy_handler)).with_state(proxy)

// Observability middleware
use ddd_bff::middleware::axum_observability::{ObservabilityState, observability_middleware};
let obs = ObservabilityState { redact_fields: Arc::new(config.redact_fields.clone()) };
app.layer(axum_mw::from_fn_with_state(obs, observability_middleware))

// Prometheus scrape endpoint
use ddd_bff::metrics::metrics_handler;
Router::new().route("/metrics", get(metrics_handler))

// Declarative OpenAPI endpoint catalogue
use ddd_bff::openapi::{ApiRoute, RouteKind, inject_routes};
inject_routes(&mut spec, API_ROUTES);

// Scalar UI + JSON spec router
use ddd_bff::openapi::openapi_router;
app.merge(openapi_router(merged_spec))

// Merge a downstream service's OpenAPI spec
use ddd_bff::openapi::merged_openapi;
let spec = merged_openapi(base_spec, &downstream_url, "/prefix").await;

// Graceful shutdown
axum::serve(listener, app)
    .with_graceful_shutdown(ddd_bff::edge::shutdown::wait_for_shutdown_signal())
    .await?;
```

**Always available** (no feature flag):

```rust
// Resilient gRPC client pool
use ddd_bff::clients::GrpcClientPool;
let pool = GrpcClientPool::from_services([("svc", url)], &ResilienceConfig::default())?;
let channel = pool.channel("svc")?;

// Body redaction
use ddd_bff::middleware::redaction::redact_json;

// Config helper
use ddd_bff::config::env_or;

// gRPC → AppError → ProblemDetail
use ddd_bff::transcode::{grpc_status_to_app_error, app_error_to_problem};
```

**admin-bff** is the reference consumer of `ddd-bff`. Under `Src/service/admin-bff/src/`:
- `config.rs` — service-specific config loading + validation; re-exports `ResilienceConfig` and uses `env_or` from `ddd_bff::config`.
- `openapi.rs` — `AdminApiDoc` (the utoipa doc for this service's schemas) + re-exports `openapi_router` from `ddd_bff::openapi`.
- `aggregation.rs` — fan-out aggregation endpoint built on `ddd_bff::prelude::*`.
- `handlers/` — REST handlers that translate to gRPC clients.
- `main.rs` — composition root: builds state, wires middleware (`observability_middleware`, `tracing_interceptor`, `audit`), mounts `/metrics`, `/openapi`, proxies, and aggregation routes, then serves with `ddd_bff::edge::shutdown::wait_for_shutdown_signal`.

Service-level code (config, handlers, aggregation, OpenAPI schemas) belongs here; anything general enough to be reused by a second BFF should move up into `ddd-bff`.

## Design References

| Document | Contents |
|----------|----------|
| `ddd-api.md` | Full specification for the `ddd-api` crate |
| `ddd-bff.md` | Full specification for the `ddd-bff` crate |
| `implementationplan.md` | Architectural overview — Clean Architecture + CQRS + BFF |
| `mediator.md` | Mediator migration plan and self-registration rationale |

## graphify

This project has a graphify knowledge graph at graphify-out/.

Rules:
- Before answering architecture or codebase questions, read graphify-out/GRAPH_REPORT.md for god nodes and community structure
- If graphify-out/wiki/index.md exists, navigate it instead of reading raw files
- After modifying code files in this session, run `graphify update .` to keep the graph current (AST-only, no API cost)
