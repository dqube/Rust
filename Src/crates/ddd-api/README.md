# ddd-api

Reusable gRPC + REST building blocks for services built on this DDD stack. Interceptors, middleware, error mapping, global exception handlers, health/readiness probes, graceful shutdown, idempotency extractors, pagination DTOs, and OpenAPI / Scalar integration.

See `../../../ddd-api.md` for the full specification — this crate implements that design.

## What's inside

### gRPC (`grpc/`)
- `server` — `GrpcServer` builder over `tonic::transport::Server` with graceful shutdown (`serve_with_shutdown`) and configurable drain timeout.
- `error` — `GrpcErrorExt` converts `AppError` → `tonic::Status` with metadata.
- `global_error_handler` — `normalise_status` enriches raw `tonic::Status` with ProblemDetail metadata; `error_mapping_interceptor` applies it as a tower layer.
- `interceptor` — auth, tracing, redaction interceptors (tower layers).
- `metadata` — `HasMetadata` + typed extractors for `Authorization`, `x-request-id`, `x-tenant-id`.
- `mapper` — `FromProto` / `IntoProto` traits + `uuid`/`timestamp` helpers.
- `pagination` — `ProtoPageInfo`, `proto_page_request`, `proto_page_response`.
- `streaming` — `TonicStream<T>` helpers.
- `validation` — `app_error_to_status` attaches `problem-details-bin` (RFC 9457 JSON) + `bad-request-bin` (google.rpc.BadRequest-compatible JSON with field violations); `GrpcValidationExt`, `GrpcValidatorRegistryExt`, `grpc_validate!`.
- `idempotency` — `extract_idempotency_key` from gRPC metadata.

### REST (`rest/`)
- `server` — `RestServer` builder over `axum::Router` with `with_graceful_shutdown` (SIGTERM/SIGINT), configurable `shutdown_timeout` (default 30s).
- `problem_details` — RFC 9457 `ProblemDetail` with `urn:problem-type:*` type URIs, `FieldViolation` (field/message/code), `ProblemDetailExt`, `IntoResponse` for `AppError`.
- `global_error_handler` — `catch_panic_layer` (CatchPanicLayer with `PanicResponseMapper` returning ProblemDetail 500), `fallback_handler` (ProblemDetail 404 for unmatched routes), `status_to_problem_detail`.
- `health` — `HealthCheck` trait (name + async check), `HealthCheckRegistry`, `health_router()` mounting `/health` (liveness, always 200) and `/ready` (readiness, runs all checks).
- `middleware` — request-id, tracing, CORS, compression layers.
- `openapi` — Scalar UI router + `openapi_json_route` + `build_openapi!`.
- `pagination` — `PageDto<T>`, `ApiResponse<T>` with `utoipa::ToSchema`.
- `validation` — `Validated<T>` extractor + `ValidatedByRegistry` + `RestValidator` + `rest_validate!`.
- `idempotency` — `IdempotencyKey` extractor (from `Idempotency-Key` header).
- `auth` *(feature `jwt`)* — `Authenticated<C>` axum extractor + `ProvideJwtValidator<C>` state trait. Validates `Authorization: Bearer <jwt>` against a generic [`JwtValidator`](../ddd-sharedkernel/src/jwt.rs) and rejects with RFC 9457 ProblemDetail (401) on failure.

### Common (`common/`)
- Pagination glue between query strings and `PageRequest`.

## Features

- `grpc` (default) — tonic + prost.
- `rest` (default) — axum + tower-http.
- `openapi` (default, requires `rest`) — utoipa + Scalar UI.
- `telemetry` — OpenTelemetry trace-context propagation.
- `jwt` — JWT bearer-token validation (REST `Authenticated<C>` extractor, gRPC `jwt_auth_interceptor`). Enables `ddd-shared-kernel/jwt`.
- `full` — all of the above.

## Examples

### REST endpoint (thin handler)

```rust
use axum::{extract::State, Json};
use ddd_api::rest::{ProblemDetail, Validated};
use ddd_application::Mediator;
use std::sync::Arc;

async fn create_order(
    State(mediator): State<Arc<Mediator>>,
    Validated(req): Validated<CreateOrderRequest>,
) -> Result<Json<OrderDto>, ProblemDetail> {
    let id = mediator.send(CreateOrder::from(req)).await
        .map_err(|e| e.to_problem_detail())?;
    Ok(Json(OrderDto { id }))
}

async fn get_order(
    State(mediator): State<Arc<Mediator>>,
    Path(id): Path<Uuid>,
) -> Result<Json<OrderDto>, ProblemDetail> {
    let order = mediator.query(GetOrder { id }).await
        .map_err(|e| e.to_problem_detail())?;
    Ok(Json(order))
}
```

### REST server with health probes + global error handling

```rust
use ddd_api::rest::{
    RestServer, health_router, HealthCheckRegistry, HealthCheck, CheckResult,
    catch_panic_layer, fallback_handler,
};

// Custom health check
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

// Assemble
let health_registry = Arc::new(HealthCheckRegistry::new());
health_registry.register(DbHealthCheck::new(db.clone()));

let app = Router::new()
    .route("/orders", post(create_order))
    .route("/orders/{id}", get(get_order))
    .merge(health_router(health_registry))  // adds /health + /ready
    .fallback(fallback_handler)              // 404 → ProblemDetail
    .layer(catch_panic_layer());             // panics → ProblemDetail 500

RestServer::new(app)
    .with_graceful_shutdown(shutdown_signal("REST"))
    .serve("0.0.0.0:8080")
    .await?;
```

### REST idempotency key

```rust
use ddd_api::rest::IdempotencyKey;

async fn create_order(
    State(mediator): State<Arc<Mediator>>,
    idempotency_key: IdempotencyKey,  // from Idempotency-Key header
    Json(req): Json<CreateOrderRequest>,
) -> Result<Json<OrderDto>, ProblemDetail> {
    let mut cmd = CreateOrder::from(req);
    cmd.idempotency_key = idempotency_key.into_inner();
    let id = mediator.send(cmd).await?;
    Ok(Json(OrderDto { id }))
}
```

### gRPC service implementation

```rust
use ddd_api::grpc::{GrpcErrorExt, FromProto};

#[tonic::async_trait]
impl OrderService for OrderServiceImpl {
    async fn create_order(
        &self,
        request: Request<CreateOrderProto>,
    ) -> Result<Response<OrderIdProto>, Status> {
        let cmd = CreateOrder::from_proto(request.into_inner());
        let id = self.mediator.send(cmd).await.to_status()?;
        Ok(Response::new(OrderIdProto { id: id.to_string() }))
    }

    async fn get_order(
        &self,
        request: Request<GetOrderProto>,
    ) -> Result<Response<OrderProto>, Status> {
        let q = GetOrder::from_proto(request.into_inner());
        let order = self.mediator.query(q).await.to_status()?;
        Ok(Response::new(order.into_proto()))
    }
}
```

### gRPC server with error interceptor + graceful shutdown

```rust
use ddd_api::grpc::{GrpcServer, error_mapping_interceptor};

let svc = OrderServiceServer::with_interceptor(
    OrderServiceImpl::new(mediator),
    error_mapping_interceptor,  // attaches problem-details-bin + bad-request-bin
);

GrpcServer::new()
    .serve_with_shutdown("[::]:50051".parse()?, shutdown_signal("gRPC"))
    .await?;
```

### gRPC idempotency key

```rust
use ddd_api::grpc::extract_idempotency_key;

async fn create_order(
    &self,
    request: Request<CreateOrderProto>,
) -> Result<Response<OrderIdProto>, Status> {
    let key = extract_idempotency_key(request.metadata())?;
    // Use key for idempotent command handling
}
```

### JWT bearer auth (feature `jwt`)

Requires `ddd-api = { features = ["jwt"] }` and `ddd-shared-kernel` with the
`jwt` feature pulled in transitively.

```rust
use std::sync::Arc;
use axum::{routing::get, Router};
use ddd_api::rest::auth::{Authenticated, ProvideJwtValidator};
use ddd_shared_kernel::jwt::{JwtValidator, StandardClaims};

#[derive(Clone)]
struct AppState {
    jwt: Arc<JwtValidator<StandardClaims>>,
}

impl ProvideJwtValidator<StandardClaims> for AppState {
    fn jwt_validator(&self) -> &JwtValidator<StandardClaims> { &self.jwt }
}

async fn me(Authenticated(claims): Authenticated<StandardClaims>) -> String {
    claims.sub
}

let state = AppState {
    jwt: Arc::new(
        JwtValidator::hs256(b"secret")
            .with_issuer(["issuer.example.com"])
            .with_audience(["my-api"])
            .with_leeway(30),
    ),
};
let app: Router = Router::new().route("/me", get(me)).with_state(state);
```

gRPC side — plug the same validator into `AuthInterceptor`:

```rust
use std::sync::Arc;
use ddd_api::grpc::auth::{jwt_auth_interceptor, JwtClaims};
use ddd_shared_kernel::jwt::{JwtValidator, StandardClaims};

let validator: Arc<JwtValidator<StandardClaims>> =
    Arc::new(JwtValidator::hs256(b"secret").with_issuer(["issuer.example.com"]));
let interceptor = jwt_auth_interceptor(validator);
let svc = OrderServiceServer::with_interceptor(order_impl, interceptor);

// Inside a handler, read the claims off the request extensions:
async fn get_order(
    &self,
    request: Request<GetOrderProto>,
) -> Result<Response<OrderProto>, Status> {
    let claims = request
        .extensions()
        .get::<JwtClaims<StandardClaims>>()
        .map(|c| c.0.clone());
    // ...
}
```

Failures map uniformly onto `AppError::Unauthorized` so the standard REST →
Problem Details (401) and gRPC → `UNAUTHENTICATED` conversions apply.

### OpenAPI / Scalar UI

```rust
use ddd_api::rest::openapi;

let app = Router::new()
    .route("/orders", post(create_order))
    .merge(openapi::scalar_ui_router("/docs"))          // Scalar UI at /docs
    .merge(openapi::openapi_json_route("/openapi.json")) // JSON spec
    ;
```

## Usage pattern

HTTP / gRPC handlers are thin — no business logic. All orchestration goes through `Mediator`.

## Error mapping

| `AppError` variant | REST | gRPC |
|---|---|---|
| `Validation` / `ValidationBatch` | 400 + ProblemDetail with `FieldViolation[]` | `INVALID_ARGUMENT` + `problem-details-bin` + `bad-request-bin` |
| `NotFound` | 404 + ProblemDetail | `NOT_FOUND` |
| `Conflict` | 409 + ProblemDetail | `ALREADY_EXISTS` |
| `Unauthorized` | 401 + ProblemDetail | `UNAUTHENTICATED` |
| `Forbidden` | 403 + ProblemDetail | `PERMISSION_DENIED` |
| `BusinessRule` | 422 + ProblemDetail | `FAILED_PRECONDITION` |
| `Internal` / `Database` | 500 + ProblemDetail | `INTERNAL` |
| Panic | 500 + ProblemDetail (via `catch_panic_layer`) | — |
| Unmatched route | 404 + ProblemDetail (via `fallback_handler`) | — |
