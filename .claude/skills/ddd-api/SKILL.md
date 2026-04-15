---
name: ddd-api
description: Guidance for the ddd-api crate — gRPC (tonic) + REST (axum) building blocks including error mapping, problem details, global exception handlers, health probes, graceful shutdown, idempotency extractors, pagination DTOs, and OpenAPI. Use when adding API endpoints, middleware, or interceptors.
---

# ddd-api

Reusable gRPC + REST building blocks for DDD services. Depends on `ddd-shared-kernel` + `ddd-application`. See `ddd-api.md` for the full specification.

## Modules

### REST (`rest/`)

| Module | Key types |
|--------|-----------|
| `server` | `RestServer` — axum server builder with graceful shutdown, configurable drain timeout |
| `problem_details` | `ProblemDetail` (RFC 9457), `FieldViolation`, `ProblemDetailExt`, `problem_type_uri()` returning `urn:problem-type:*` URIs |
| `global_error_handler` | `catch_panic_layer` (CatchPanicLayer), `fallback_handler` (404 ProblemDetail), `status_to_problem_detail`, `PanicResponseMapper` |
| `health` | `HealthCheck` trait, `HealthCheckRegistry`, `health_router()` mounting `/health` + `/ready` |
| `error` | `RestErrorResponse` — `AppError` → `ProblemDetail` `IntoResponse` |
| `middleware` | Request-id, tracing, CORS, compression layers |
| `validation` | `Validated<T>` extractor, `ValidatedByRegistry`, `RestValidator`, `rest_validate!` |
| `idempotency` | `IdempotencyKey` extractor (from `Idempotency-Key` header) |
| `pagination` | `PageDto<T>`, `ApiResponse<T>` with `utoipa::ToSchema` |
| `openapi` | Scalar UI router, `openapi_json_route`, `build_openapi!` |

### gRPC (`grpc/`)

| Module | Key types |
|--------|-----------|
| `server` | `GrpcServer` — tonic server builder with graceful shutdown |
| `error` | `GrpcErrorExt` — `AppError` → `tonic::Status` with metadata |
| `global_error_handler` | `normalise_status`, `error_mapping_interceptor` — attaches `problem-details-bin` + `bad-request-bin` metadata |
| `validation` | `app_error_to_status`, `GrpcValidationExt`, `GrpcValidatorRegistryExt`, `grpc_validate!` |
| `interceptor` | Auth, tracing, redaction interceptors (tower layers) |
| `metadata` | `HasMetadata` + typed extractors for `Authorization`, `x-request-id`, `x-tenant-id` |
| `mapper` | `FromProto`, `IntoProto` traits + uuid/timestamp helpers |
| `pagination` | `ProtoPageInfo`, `proto_page_request`, `proto_page_response` |
| `streaming` | `TonicStream<T>` helpers |
| `idempotency` | `extract_idempotency_key` from gRPC metadata |

### Common (`common/`)

| Module | Key types |
|--------|-----------|
| `pagination` | Pagination glue between query-string and `PageRequest` |

## Feature flags

- `grpc` (default) — tonic + prost.
- `rest` (default) — axum + tower-http.
- `openapi` (default, requires `rest`) — utoipa + Scalar UI.
- `telemetry` — OpenTelemetry trace-context propagation.
- `full` — all of the above.

## Recipes

### Adding a REST endpoint
```rust
use axum::{extract::State, Json};
use ddd_api::rest::{ProblemDetail, Validated};
use ddd_application::Mediator;
use std::sync::Arc;

async fn create_order(
    State(mediator): State<Arc<Mediator>>,
    Validated(req): Validated<CreateOrderRequest>,
) -> Result<Json<OrderDto>, ProblemDetail> {
    let id = mediator.send(CreateOrder::from(req)).await?;
    Ok(Json(OrderDto { id }))
}
```

### Wiring a REST server with health + error handling
```rust
use ddd_api::rest::{
    RestServer, health_router, HealthCheckRegistry,
    catch_panic_layer, fallback_handler,
};

let health_registry = HealthCheckRegistry::new();
health_registry.register(DbHealthCheck::new(db.clone()));

let app = Router::new()
    .route("/orders", post(create_order))
    .merge(health_router(Arc::new(health_registry)))
    .fallback(fallback_handler)
    .layer(catch_panic_layer());

RestServer::new(app)
    .with_graceful_shutdown(shutdown_signal("REST"))
    .serve("0.0.0.0:8080")
    .await?;
```

### Adding a gRPC service
```rust
use ddd_api::grpc::{GrpcServer, GrpcErrorExt, FromProto};

#[tonic::async_trait]
impl OrderService for OrderServiceImpl {
    async fn create_order(
        &self, request: Request<CreateOrderProto>,
    ) -> Result<Response<OrderIdProto>, Status> {
        let cmd = CreateOrder::from_proto(request.into_inner());
        let id = self.mediator.send(cmd).await.to_status()?;
        Ok(Response::new(OrderIdProto { id: id.to_string() }))
    }
}
```

### Wiring a gRPC server with error interceptor
```rust
use ddd_api::grpc::{GrpcServer, error_mapping_interceptor};

let svc = OrderServiceServer::with_interceptor(
    OrderServiceImpl::new(mediator),
    error_mapping_interceptor,
);

GrpcServer::new()
    .serve_with_shutdown(addr, shutdown_signal("gRPC"))
    .await?;
```

### Using idempotency keys
```rust
// REST: extract from Idempotency-Key header
use ddd_api::rest::IdempotencyKey;

async fn create_order(
    State(mediator): State<Arc<Mediator>>,
    idempotency_key: IdempotencyKey,
    Json(req): Json<CreateOrderRequest>,
) -> Result<Json<OrderDto>, ProblemDetail> { /* ... */ }

// gRPC: extract from metadata
use ddd_api::grpc::extract_idempotency_key;

let key = extract_idempotency_key(request.metadata())?;
```

### Health probes
```rust
use ddd_api::rest::health::{HealthCheck, CheckResult};

struct DbHealthCheck { db: DatabaseConnection }

#[async_trait]
impl HealthCheck for DbHealthCheck {
    fn name(&self) -> &str { "database" }
    async fn check(&self) -> CheckResult {
        match self.db.ping().await {
            Ok(_) => CheckResult::healthy(),
            Err(e) => CheckResult::unhealthy(e.to_string()),
        }
    }
}
```

## Rules

- No business logic in REST/gRPC handlers — all orchestration goes through `Mediator`.
- `AppError` maps to `ProblemDetail` (REST) or `tonic::Status` with metadata (gRPC) automatically.
- Graceful shutdown listens for SIGTERM/SIGINT with configurable drain timeout (default: 30s).
- Global error handlers catch panics + unmatched routes and return structured error responses.
