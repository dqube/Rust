# ddd-api

Reusable gRPC + REST building blocks for services built on this DDD stack. Interceptors, middleware, error mapping, global exception handlers, health/readiness probes, graceful shutdown, idempotency extractors, pagination DTOs, and OpenAPI / Scalar integration.

## Standalone Examples

For full implementation details, see:
- [`rest_server_full.rs`](examples/rest_server_full.rs) — Complete Axum server with validation and RFC 9457 error mapping.
- [`grpc_server_full.rs`](examples/grpc_server_full.rs) — Tonic server setup with error-mapping interceptors and graceful shutdown.

## What's inside

### gRPC (`grpc/`)
- `server` — `GrpcServer` builder with graceful shutdown and configurable drain timeout.
- `error` — `GrpcErrorExt` converts `AppError` → `tonic::Status` with metadata.
- `interceptor` — auth, tracing, redaction interceptors.
- `metadata` — typed extractors for `Authorization`, `x-request-id`.
- `validation` — attaches `problem-details-bin` (RFC 9457 JSON) + `bad-request-bin`.

### REST (`rest/`)
- `server` — `RestServer` builder with `with_router(app)` and `run()` (SIGTERM/SIGINT).
- `problem_details` — RFC 9457 `ProblemDetail` with `urn:problem-type:*` type URIs.
- `health` — `HealthCheckRegistry` mounting `/health` and `/ready`.
- `openapi` — Scalar UI router + `merged_openapi`.
- `validation` — `Validated<T>` extractor + `RestValidator`.
- `idempotency` — `IdempotencyKey` extractor (from `Idempotency-Key` header).
- `auth` *(feature `jwt`)* — `Authenticated<C>` axum extractor. Validates `Authorization: Bearer <jwt>` against a generic `JwtValidator`.

## Examples

### REST server with health probes + global error handling

```rust
use ddd_api::rest::{RestServer, health_router, HealthCheckRegistry, catch_panic_layer, fallback_handler};

// Assemble
let health_registry = Arc::new(HealthCheckRegistry::new());
health_registry.register(DbHealthCheck::new(db.clone()));

let app = Router::new()
    .route("/orders", post(create_order))
    .merge(health_router(health_registry))
    .fallback(fallback_handler)
    .layer(catch_panic_layer());

RestServer::new()
    .with_router(app)
    .run()
    .await?;
```

### gRPC server with error interceptor + graceful shutdown

```rust
use ddd_api::grpc::{GrpcServer, error_mapping_interceptor};

let router = Server::builder()
    .layer(tonic::middleware::interceptor(error_mapping_interceptor))
    .add_service(OrderServiceServer::new(order_impl));

GrpcServer::new()
    .with_router(router)
    .run()
    .await?;
```

## Error mapping

| `AppError` variant | REST | gRPC |
|---|---|---|
| `Validation` / `ValidationBatch` | 400 + ProblemDetail with `FieldViolation[]` | `INVALID_ARGUMENT` + `problem-details-bin` + `bad-request-bin` |
| `NotFound` | 404 + ProblemDetail | `NOT_FOUND` |
| `Conflict` | 409 + ProblemDetail | `ALREADY_EXISTS` |
| `BusinessRule` | 422 + ProblemDetail | `FAILED_PRECONDITION` |
| `Internal` / `Database` | 500 + ProblemDetail | `INTERNAL` |
| Unmatched route | 404 + ProblemDetail (via `fallback_handler`) | — |
