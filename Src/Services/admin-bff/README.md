# admin-bff

REST gateway for the admin console. Fronts two backend services with a
single JSON surface, OpenAPI documentation, and Prometheus metrics.

```
                 ┌──────────────────────────────────────────────────┐
  Browser  ──►   │ admin-bff (axum, port 3001)                      │
                 │                                                  │
                 │   /admin/products/*   ──gRPC──►  product-service  │
                 │   /admin/orders/*     ──gRPC──►  order-service   │
                 │   /admin/orders/batch   fan-out ─► order-service │
                 │   /admin/catalog/summary  fan-out ─► product-svc │
                 │                                                  │
                 │   /scalar  /api-docs/openapi.json                │
                 │   /metrics  /health                              │
                 └──────────────────────────────────────────────────┘
```

Both the product side and the order side talk gRPC to their respective
services via `tonic` clients sourced from a `ddd_bff::clients::GrpcClientPool`.

---

## What it reuses from `ddd-bff`

This service is an example of consuming
[`ddd-bff`](../../crates/ddd-bff) as a library in **pick-and-mix** style
(axum-response feature). Most generic concerns are thin re-exports:

| Concern                        | Source in ddd-bff                                         |
|--------------------------------|-----------------------------------------------------------|
| Resilient gRPC client pool     | `ddd_bff::clients::{GrpcClientPool, ResilientChannel}`    |
| Resilience config + `env_or`   | `ddd_bff::config::{ResilienceConfig, env_or}`             |
| Body redaction                 | `ddd_bff::middleware::redaction::redact_json`             |
| Axum observability middleware  | `ddd_bff::middleware::axum_observability::{ObservabilityState, observability_middleware}` |
| Audit logging                  | `ddd_bff::middleware::audit::{audit, AuditEvent}`         |
| W3C Trace context propagation  | `ddd_bff::middleware::tracing_interceptor::TracingInterceptor` |
| Prometheus metrics handler     | `ddd_bff::metrics::{BFF_METRICS, metrics_handler}`        |
| OpenAPI endpoint catalogue     | `ddd_bff::openapi::{ApiRoute, RouteKind, inject_routes, …}` |
| Scalar UI + spec router        | `ddd_bff::openapi::openapi_router`                        |
| Downstream spec merge          | `ddd_bff::openapi::merged_openapi`                        |
| HTTP reverse proxy             | `ddd_bff::proxy::{ProxyState, proxy_handler}`             |
| Graceful shutdown              | `ddd_bff::edge::shutdown::wait_for_shutdown_signal`       |
| `tonic::Status` → `AppError`   | `ddd_bff::transcode::grpc_status_to_app_error`            |

Service-specific code: `AdminApiDoc` schema list, gRPC handlers
(`ProductClient`, `OrderClient`), aggregation logic, route wiring,
local `ProblemDetail` (with `utoipa::ToSchema`), and configuration.

---

## Layout

```
src/
├── main.rs              # boots axum, wires routes, API_ROUTES table
├── lib.rs
├── config.rs            # AdminBffConfig (service URLs + re-exports ResilienceConfig)
├── aggregation.rs       # /admin/orders/batch gRPC fan-out
├── openapi.rs           # AdminApiDoc + re-exports openapi_router
├── proto_product.rs     # tonic generated code for product-service
├── proto_order.rs       # tonic generated code for order-service
└── handlers/
    ├── mod.rs
    ├── error.rs         # local ProblemDetail conversion
    ├── products.rs      # ProductClient (tonic) + axum handlers
    ├── orders.rs        # OrderClient (tonic) + axum handlers
    └── aggregation.rs   # /admin/catalog/summary
```

---

## Endpoints

### Products (gRPC pass-through)

| Method | Path                                       | gRPC method                                |
|--------|--------------------------------------------|--------------------------------------------|
| POST   | `/admin/products`                          | `ProductService/CreateProduct`             |
| GET    | `/admin/products`                          | `ProductService/ListProducts`              |
| GET    | `/admin/products/{id}`                     | `ProductService/GetProduct`                |
| PUT    | `/admin/products/{id}/stock`               | `ProductService/UpdateStock`               |
| PUT    | `/admin/products/{id}/deactivate`          | `ProductService/DeactivateProduct`         |
| POST   | `/admin/products/{id}/image-upload-url`    | `ProductService/RequestImageUploadUrl`     |
| POST   | `/admin/products/{id}/confirm-image`       | `ProductService/ConfirmImageUpload`        |

### Orders (gRPC pass-through + aggregation)

| Method | Path                          | Behaviour                               |
|--------|-------------------------------|-----------------------------------------|
| POST   | `/admin/orders`               | `OrderService/CreateOrder`              |
| GET    | `/admin/orders`               | `OrderService/ListOrders`               |
| GET    | `/admin/orders/{id}`          | `OrderService/GetOrder`                 |
| PUT    | `/admin/orders/{id}/confirm`  | `OrderService/ConfirmOrder`             |
| PUT    | `/admin/orders/{id}/cancel`   | `OrderService/CancelOrder`              |
| POST   | `/admin/orders/batch`         | Parallel gRPC fan-out (partial failure OK) |

### Aggregation

| Method | Path                      | Behaviour                               |
|--------|---------------------------|-----------------------------------------|
| GET    | `/admin/catalog/summary`  | Product list + counts via gRPC          |

### Operational

| Method | Path                     | Purpose                                 |
|--------|--------------------------|-----------------------------------------|
| GET    | `/health`                | Liveness                                |
| GET    | `/metrics`               | Prometheus text format                  |
| GET    | `/scalar`                | Scalar UI for the merged OpenAPI spec   |
| GET    | `/api-docs/openapi.json` | Merged OpenAPI 3.x spec                 |

---

## OpenAPI spec

The merged spec is built at startup:

1. `AdminApiDoc::openapi()` generates a base spec from all registered `utoipa::ToSchema` types.
2. `merged_openapi(base, downstream_url, "/admin/orders")` fetches the
   order-service spec and merges its paths + schemas (non-destructive).
3. `inject_routes(&mut spec, API_ROUTES)` writes every endpoint's path item,
   operation id, parameters, request/response `$ref`s, and
   `x-bff-kind` / `x-bff-upstream` / `x-bff-grpc-method` extensions.

### Adding a new endpoint

Add one entry to `API_ROUTES` in `main.rs`:

```rust
ApiRoute {
    kind: RouteKind::Passthrough {
        upstream: "product",
        grpc_method: "product.v1.ProductService/NewRpc",
    },
    method: "POST",
    path: "/admin/products/{id}/new-action",
    operation_id: "new_action",
    summary: "Perform new action on a product",
    tag: "Products",
    params: &[Param { name: "id", location: "path", required: true,
                      schema_type: "string", description: "Product UUID" }],
    request_body: Some(SchemaRef { name: "NewActionRequest", content_type: "application/json" }),
    responses: &[ResponseSpec { status: 200, description: "OK", schema: None }],
}
```

No per-handler `#[utoipa::path]` attribute needed.

---

## Observability

`observability_middleware` (from ddd-bff) wraps every request:

- captures `X-Forwarded-For` Client IPs,
- generates / propagates `x-request-id` (UUID v7),
- parses W3C `traceparent` context and propagates deeply into gRPC wrappers,
- logs method, path, status, and latency (and optionally request/response bodies when `LOG_REQUEST_BODIES=true` and TRACE is DEBUG),
- redacts request bodies using `redact_json` against `REDACT_FIELDS`,
- updates Prometheus metrics via `BFF_METRICS`:
  - `bff_http_requests_total{route,method,status}`
  - `bff_http_request_duration_seconds{route,method}`
  - `bff_http_requests_in_flight`

A `CatchPanicLayer` converts panics into a 500 ProblemDetail.
A global axum `TimeoutLayer` enforces hard per-request deadlines across all handlers (`REQUEST_TIMEOUT_SECS`).
Mutating endpoints (e.g. `create_product`) additionally emit strict structured JSON logs using the dedicated `"audit"` tracing target to separate out write events.

---

## Errors

All non-2xx responses are `application/problem+json` (RFC 9457):

```json
{
  "type":   "urn:problem-type:not-found",
  "title":  "Not Found",
  "status": 404,
  "detail": "Order abc-123 not found",
  "instance": "/admin/orders/abc-123"
}
```

`ddd_bff::transcode::grpc_status_to_app_error` maps `tonic::Code` → `AppError`;
the local `app_error_to_problem` in `handlers/error.rs` shapes it for utoipa.

---

## Configuration

Set environment variables (or place them in a `.env` file):

| Variable                   | Default              | Purpose                              |
|----------------------------|----------------------|--------------------------------------|
| `ADMIN_BFF_HOST`           | `0.0.0.0`            | Bind host                            |
| `ADMIN_BFF_PORT`           | `3001`               | Bind port                            |
| `PRODUCT_SERVICE_URL`      | `http://localhost:50052` | product-service gRPC address     |
| `ORDER_SERVICE_URL`        | `http://localhost:50051` | order-service gRPC address       |
| `GRPC_TIMEOUT_MS`          | `5000`               | Per-call timeout                     |
| `GRPC_MAX_CONCURRENT`      | `100`                | Per-channel concurrency cap          |
| `REDACT_FIELDS`            | `password,secret,token,authorization` | Logged-payload redaction list |
| `REQUEST_TIMEOUT_SECS`     | `30`                 | Axum HTTP tower timeout configuration|
| `LOG_REQUEST_BODIES`       | `false`              | Enables logging payload bodies (when tracing level is DEBUG) |
| `JWT_SECRET`               | _unset_              | HS256 signing secret. When set, all `/admin/*` routes require `Authorization: Bearer <jwt>`. When empty, auth is disabled (dev mode) and a warning is logged. |
| `JWT_ISSUER`               | _unset_              | Expected `iss` claim (optional)      |
| `JWT_AUDIENCE`             | `admin-bff`          | Expected `aud` claim                 |
| `JWT_LEEWAY_SECS`          | `30`                 | Clock skew tolerance for `exp`/`nbf` |

---

## Build / run

```bash
# Check
cargo check --manifest-path Src/Services/admin-bff/Cargo.toml

# Build
cargo build --manifest-path Src/Services/admin-bff/Cargo.toml

# Run (requires product-service on :50052 and order-service on :50051)
cargo run --manifest-path Src/Services/admin-bff/Cargo.toml

# Scalar UI
open http://localhost:3001/scalar
```
