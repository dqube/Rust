---
name: ddd-bff
description: Guidance for the ddd-bff crate — Backend for Frontend library providing reusable BFF building blocks (gRPC client pool, HTTP proxy, observability middleware, Prometheus metrics, OpenAPI/Scalar) consumed by service binaries like admin-bff. Use when adding BFF endpoints, aggregation handlers, gRPC client wiring, observability middleware, or OpenAPI catalogue entries.
---

# ddd-bff

**Library crate** — reusable BFF building blocks. NOT a binary. Depends only on `ddd-shared-kernel`. Service binaries (e.g. `admin-bff`) consume it in pick-and-mix style via the `axum-response` feature flag.

## Feature flags

| Feature | What it enables |
|---------|----------------|
| (default) | `clients`, `config`, `edge`, `metrics` (counters/histograms only), `middleware::redaction`, `transcode` |
| `axum-response` | `proxy`, `openapi::router`, `openapi::merge`, `middleware::axum_observability`, `metrics::metrics_handler` |

## Modules

### Clients (`clients/`)

| Module | Key types |
|--------|-----------|
| `pool` | `GrpcClientPool` — HashMap of service name → `ResilientChannel`, built from `BffConfig` |
| `resilient_client` | `ResilientChannel` — tonic `Channel` wrapper with timeout + concurrency-limit config |

### Config (`config.rs`)

| Symbol | Purpose |
|--------|---------|
| `BffConfig` | Root config struct populated from env |
| `ServiceUrls` | Named service URL map |
| `ResilienceConfig` | Timeout, concurrency, retry, circuit-breaker settings |
| `env_or(key, default)` | `std::env::var` helper with fallback |

### Edge (`edge/`)

| Module | Key types |
|--------|-----------|
| `shutdown` | `wait_for_shutdown_signal()` — SIGTERM/SIGINT future for `axum::serve(...).with_graceful_shutdown(...)` |

### Metrics (`metrics.rs`)

| Symbol | Purpose |
|--------|---------|
| `BFF_METRICS` | `lazy_static` Prometheus singleton with counters, histograms, gauge |
| `metrics_handler()` | axum handler for `/metrics` scrape endpoint (`axum-response` feature) |

Metric names: `bff_http_requests_total`, `bff_http_request_duration_seconds`, `bff_http_requests_in_flight`, `bff_upstream_requests_total`, `bff_upstream_request_duration_seconds`.

### Middleware (`middleware/`)

| Module | Key types |
|--------|-----------|
| `redaction` | `redact_json()`, `redact_json_string()` — recursive case-insensitive JSON field redaction |
| `axum_observability` (`axum-response`) | `ObservabilityState { redact_fields }`, `observability_middleware` — logs method/path/redacted body, updates `BFF_METRICS` |

### Proxy (`proxy.rs`) — `axum-response` feature

| Symbol | Purpose |
|--------|---------|
| `ProxyState` | `{ client, upstream_base: Arc<String>, strip_prefix: Arc<String> }` |
| `ProxyState::new(upstream_base, strip_prefix, timeout)` | Build proxy state |
| `proxy_handler` | axum handler: forwards any HTTP method to upstream, strips prefix |

### OpenAPI (`openapi/`) — `axum-response` feature

| Symbol | Purpose |
|--------|---------|
| `ApiRoute` | Declarative route descriptor (path, method, tag, summary, params, response) |
| `RouteKind` | `Proxy` / `Grpc` / `Aggregated` |
| `Param`, `SchemaRef`, `ResponseSpec` | Supporting types for `ApiRoute` |
| `inject_routes(&mut spec, routes)` | Merge an `&[ApiRoute]` array into an OpenAPI `Value` |
| `openapi_router(spec: Value) -> Router` | axum router: Scalar at `/scalar`, JSON spec at `/api-docs/openapi.json` |
| `merged_openapi(base, url, prefix)` | Async: fetch downstream OpenAPI JSON from `url` and merge into `base` under `prefix` |

### Transcode (`transcode/`)

| Symbol | Purpose |
|--------|---------|
| `grpc_status_to_app_error(Status) -> AppError` | Map tonic status codes to `AppError` variants |
| `app_error_to_problem(AppError) -> ProblemDetail` | Convert `AppError` to RFC 9457 `ProblemDetail` |

## Recipes

### Wire up a new BFF service binary

```rust
// Cargo.toml
[dependencies]
ddd-bff = { path = "../../../Src/crates/ddd-bff", features = ["axum-response"] }

// main.rs
use ddd_bff::{
    clients::GrpcClientPool,
    config::{BffConfig, env_or},
    edge::shutdown::wait_for_shutdown_signal,
    metrics::metrics_handler,
    middleware::axum_observability::{ObservabilityState, observability_middleware},
    openapi::{openapi_router, inject_routes},
    proxy::{ProxyState, proxy_handler},
};
```

### Adding a gRPC pass-through handler

1. Call `pool.channel("service_name")` to get a tonic `Channel`.
2. Instantiate the generated proto client.
3. Convert errors with `grpc_status_to_app_error` → `app_error_to_problem`.
4. Register the route on the axum `Router`.
5. Add an `ApiRoute` entry to the service's `API_ROUTES` constant and call `inject_routes`.

```rust
pub async fn get_order(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<OrderResponse>, ProblemDetail> {
    use ddd_bff::transcode::{grpc_status_to_app_error, app_error_to_problem};
    let ch = state.pool.channel("order-service")
        .map_err(|e| app_error_to_problem(e))?;
    let mut client = OrderServiceClient::new(ch);
    let resp = client.get_order(GetOrderRequest { id })
        .await
        .map_err(|s| app_error_to_problem(grpc_status_to_app_error(s)))?;
    Ok(Json(resp.into_inner().into()))
}
```

### Mounting the reverse HTTP proxy

```rust
use ddd_bff::proxy::{ProxyState, proxy_handler};
use std::time::Duration;

let proxy = ProxyState::new(
    "http://product-service:8082".into(),
    "/products".into(),
    Duration::from_secs(5),
);
Router::new()
    .route("/products/{*path}", any(proxy_handler))
    .with_state(proxy)
```

### Observability middleware

```rust
use ddd_bff::middleware::axum_observability::{ObservabilityState, observability_middleware};
use std::sync::Arc;

let obs = ObservabilityState {
    redact_fields: Arc::new(vec!["password".into(), "token".into()]),
};
app.layer(axum::middleware::from_fn_with_state(obs, observability_middleware))
```

### Declarative OpenAPI catalogue

```rust
use ddd_bff::openapi::{ApiRoute, RouteKind, inject_routes};

const API_ROUTES: &[ApiRoute] = &[
    ApiRoute {
        path: "/orders/{id}",
        method: "GET",
        tag: "Orders",
        summary: "Get order by ID",
        kind: RouteKind::Grpc,
        params: &[],
        response: None,
    },
];

// In startup:
inject_routes(&mut spec, API_ROUTES);
app.merge(openapi_router(spec))
```

### Merging a downstream spec

```rust
use ddd_bff::openapi::merged_openapi;

let spec = merged_openapi(base_spec, "http://product-service:8082", "/products").await;
inject_routes(&mut spec, API_ROUTES);
app.merge(openapi_router(spec))
```

### Graceful shutdown

```rust
axum::serve(listener, app)
    .with_graceful_shutdown(ddd_bff::edge::shutdown::wait_for_shutdown_signal())
    .await?;
```

### Configuring resilience

Set env vars (all have sensible defaults):

| Variable | Default | Description |
|----------|---------|-------------|
| `GRPC_TIMEOUT_MS` | 5000 | Per-call timeout |
| `GRPC_MAX_CONCURRENT` | 100 | Max in-flight per service |
| `GRPC_RETRY_MAX_ATTEMPTS` | 3 | Retry ceiling |
| `GRPC_CB_FAILURE_THRESHOLD` | 5 | Circuit breaker opens after N failures |
| `GRPC_CB_SUCCESS_THRESHOLD` | 2 | Circuit breaker closes after N successes |
| `GRPC_CB_TIMEOUT_SECS` | 30 | Half-open delay |

## admin-bff — reference consumer

`admin-bff` under `Src/Services/admin-bff/` is the canonical example. All its modules are thin re-exports:

| File | Re-exports from |
|------|----------------|
| `api_routes.rs` | `ddd_bff::openapi::{inject_routes, ApiRoute, Param, ResponseSpec, RouteKind, SchemaRef}` |
| `middleware.rs` | `ddd_bff::middleware::axum_observability::{observability_middleware, ObservabilityState}` + `redact_json` |
| `metrics.rs` | `ddd_bff::metrics::{metrics_handler, BFF_METRICS}` |
| `proxy.rs` | `ddd_bff::proxy::{proxy_handler, ProxyState}` + `ddd_bff::openapi::merged_openapi` |
| `openapi.rs` | `AdminApiDoc` (service-specific schema list) + `ddd_bff::openapi::openapi_router` |
| `config.rs` | uses `ddd_bff::config::env_or` — no local `fn env_or` |

Service-specific logic (aggregation handlers, route table, `AppState`) lives in `admin-bff` only.

## Rules

- `ddd-bff` is a **library** — never add a `main.rs` or `[[bin]]` target here.
- Never add `ddd-domain` or `ddd-application` as deps — `ddd-bff` depends only on `ddd-shared-kernel`.
- Never add `ddd-infrastructure` or `ddd-api` as deps.
- All axum-specific code (handlers, middleware, routes) must be behind the `axum-response` feature.
- Do not put business logic inside `ddd-bff` — delegate to downstream gRPC services from the consuming binary.
