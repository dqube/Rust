# ddd-bff

Reusable building blocks for **Backend-for-Frontend** gateways.

`ddd-bff` is a **library crate**. It provides the pieces needed to stand
up a REST/JSON gateway in front of a fleet of gRPC services without coupling
to any specific service. Consumers supply their own `main.rs`, downstream
service URLs, and — when using transcoding — their own protobuf descriptor pool.

`ddd-bff` depends only on `ddd-shared-kernel`. It contains no business logic
and no per-service code generation.

```
┌────────┐   REST/JSON  ┌───────────────┐   gRPC   ┌──────────────────┐
│ Client │ ───────────► │ your bff bin  │ ───────► │ service A / B …  │
└────────┘ ◄─────────── │  (uses        │ ◄─────── │   gRPC backends  │
            JSON / SSE  │   ddd-bff)    │          └──────────────────┘
                        └───────────────┘
                              │
                              ├─► Prometheus  (/metrics)
                              └─► OTLP traces (optional)
```

---

## What's in the box

| Module                         | What it gives you                                                      |
|--------------------------------|------------------------------------------------------------------------|
| `clients`                      | `GrpcClientPool` keyed by service name + `ResilientChannel`            |
| `config`                       | Generic `BffConfig` (host, timeouts, resilience, redaction, OTLP, …)   |
| `edge`                         | `routes.yaml` router, hyper service, accept loop, observability        |
| `edge::shutdown`               | `wait_for_shutdown_signal()` — SIGTERM/SIGINT handler                  |
| `edge::metrics_server`         | Loopback `/metrics` + `/health` hyper server                           |
| `transcode`                    | REST↔gRPC conversion, `BytesCodec`, SSE streaming, error mapping       |
| `metrics`                      | `BFF_METRICS` Prometheus singleton + `metrics_handler` axum handler    |
| `middleware::redaction`        | `redact_json` / `redact_json_string` for logged payloads               |
| `middleware::axum_observability`| `ObservabilityState` + `observability_middleware` for axum (axum-response) |
| `middleware::tracing_interceptor` | `TracingInterceptor` gRPC propagation of task-local traces (axum-response)|
| `middleware::audit`            | `audit()` logger + `AuditEvent` definition (axum-response)             |
| `middleware::axum_auth`        | `jwt_auth_layer` + `Authenticated<C>` — JWT bearer validation (feature `jwt`) |
| `openapi::api_routes`          | `ApiRoute`, `RouteKind`, `inject_routes` — declarative endpoint catalogue |
| `openapi::router`              | `openapi_router(spec)` — Scalar UI + JSON spec axum router (axum-response) |
| `openapi::merge`               | `merged_openapi()` — fetch + merge a downstream OpenAPI spec (axum-response) |
| `proxy`                        | `ProxyState` + `proxy_handler` — generic HTTP reverse proxy (axum-response) |

---

## Features

| Feature        | Enables                                         |
|----------------|-------------------------------------------------|
| _(default)_    | `clients`, `edge`, `transcode`, `metrics`, `middleware::redaction` |
| `axum-response`| `middleware::axum_observability`, `openapi::router`, `openapi::merge`, `proxy` |
| `jwt`          | `middleware::axum_auth` (implies `axum-response`; enables `ddd-shared-kernel/jwt`) |

---

## Integration styles

### Pick-and-mix (axum-based BFF)

Use individual primitives inside an axum app. This is what
[`admin-bff`](../../service/admin-bff) does:

```rust
use ddd_bff::clients::GrpcClientPool;
use ddd_bff::config::{env_or, ResilienceConfig};
use ddd_bff::middleware::axum_observability::{ObservabilityState, observability_middleware};
use ddd_bff::metrics::{metrics_handler, BFF_METRICS};
use ddd_bff::openapi::{inject_routes, merged_openapi, openapi_router, ApiRoute};
use ddd_bff::proxy::{ProxyState, proxy_handler};

// gRPC client pool
let pool = GrpcClientPool::from_services(
    [("product", "http://product-service:50052")],
    &ResilienceConfig::default(),
)?;
let channel = pool.channel("product")?;

// HTTP reverse proxy state
let proxy = ProxyState::new(
    "http://order-service:8080".into(),
    "/admin/orders".into(),
    std::time::Duration::from_secs(5),
);

// Observability middleware
let obs = ObservabilityState {
    redact_fields: Arc::new(vec!["password".into()]),
};

// Build + serve with graceful shutdown
let app = Router::new()
    .route("/admin/orders/{*path}", any(proxy_handler))
    .route("/metrics", get(metrics_handler))
    .merge(openapi_router(merged_spec))
    .layer(axum_mw::from_fn_with_state(obs, observability_middleware))
    .with_state(proxy);

axum::serve(listener, app)
    .with_graceful_shutdown(ddd_bff::edge::shutdown::wait_for_shutdown_signal())
    .await?;
```

### JWT bearer auth (feature `jwt`)

Protect a subtree of routes with generic JWT validation. The validator comes
from `ddd-shared-kernel::jwt` and is framework-agnostic — plug in any claims
struct, any supported algorithm (HS256/384/512, RS256, ES256), and optional
`iss`/`aud`/`sub`/leeway constraints.

```rust
use std::sync::Arc;
use axum::{middleware as axum_mw, routing::get, Extension, Router};
use ddd_bff::middleware::axum_auth::jwt_auth_layer;
use ddd_shared_kernel::jwt::{JwtValidator, StandardClaims};

let validator: Arc<JwtValidator<StandardClaims>> = Arc::new(
    JwtValidator::hs256(b"shared-secret")
        .with_issuer(["https://issuer.example.com"])
        .with_audience(["admin-bff"])
        .with_leeway(30),
);

async fn me(Extension(claims): Extension<StandardClaims>) -> String {
    claims.sub
}

// Guard a subtree — /health and /metrics stay open
let protected = Router::new()
    .route("/admin/me", get(me))
    .layer(axum_mw::from_fn_with_state(
        validator.clone(),
        jwt_auth_layer::<StandardClaims>,
    ));

let app = Router::new()
    .merge(protected)
    .route("/health",  get(|| async { "ok" }))
    .route("/metrics", get(ddd_bff::metrics::metrics_handler));
```

Handlers can read the claims with `Extension<C>` (shown above) or with the
extractor `ddd_bff::middleware::axum_auth::Authenticated<C>`. Missing /
malformed / expired tokens return RFC 9457 Problem Details (401). Custom
claim structs work by substituting `StandardClaims` for any `Deserialize`
type — see the crate docs.

### Full edge (routes.yaml + transcoding)

For a YAML-driven REST↔gRPC gateway with no per-route code:

```rust
use ddd_bff::config::BffConfig;
use ddd_bff::edge;
use ddd_bff::transcode;
use tokio_util::sync::CancellationToken;

let config = BffConfig::from_env();

// 1. Install your descriptor pool (compiled from your services' .proto).
let pool = transcode::decode_pool(MY_DESCRIPTOR_BYTES)?;
transcode::install(pool)?;

// 2. Build the edge from routes.yaml.
let routes   = edge::route_config::load(&config.routes_path)?;
let upstream = edge::upstream::UpstreamRegistry::from_config(&routes, &config.resilience)?;
let router   = edge::router::Router::compile(&routes)?;
let svc      = edge::service::BffEdge::new(router, upstream, config.redact_fields.clone());

// 3. Wire shutdown, /metrics, accept loop.
let cancel = CancellationToken::new();
edge::shutdown::install_signal_handler(cancel.clone());

tokio::join!(
    edge::server::run(svc, &config, cancel.clone()),
    edge::metrics_server::run(&config, cancel.clone()),
);
```

---

## OpenAPI catalogue

`openapi::api_routes` provides a declarative table for describing BFF
endpoints without per-handler `#[utoipa::path]` attributes. One entry per
endpoint; `inject_routes` writes the path item, operation, parameters, request
body, and responses into the spec in a single pass:

```rust
use ddd_bff::openapi::{inject_routes, ApiRoute, Param, ResponseSpec, RouteKind, SchemaRef};

const ROUTES: &[ApiRoute] = &[
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "product",
            grpc_method: "product.v1.ProductService/GetProduct",
        },
        method: "GET",
        path: "/products/{id}",
        operation_id: "get_product",
        summary: "Get a product by id",
        tag: "Products",
        params: &[Param {
            name: "id",
            location: "path",
            required: true,
            schema_type: "string",
            description: "Product UUID",
        }],
        request_body: None,
        responses: &[ResponseSpec { status: 200, description: "OK",
            schema: Some(SchemaRef { name: "GetProductResponse", content_type: "application/json" }) }],
    },
];

inject_routes(&mut spec, ROUTES);
```

`RouteKind` variants:

| Variant | `x-bff-kind` | Behaviour |
|---------|-------------|-----------|
| `Passthrough { upstream, grpc_method }` | `passthrough_unary` | REST → one gRPC call |
| `Aggregation` | `aggregation` | Fan-out to multiple backends |
| `Stream { upstream, grpc_method }` | `passthrough_stream` | gRPC stream → SSE |

---

## Observability (axum)

The `axum_observability` middleware (requires `axum-response`) tracks these
Prometheus metrics via `BFF_METRICS`:

| Metric | Labels | Type |
|--------|--------|------|
| `bff_http_requests_total` | `route`, `method`, `status` | Counter |
| `bff_http_request_duration_seconds` | `route`, `method` | Histogram |
| `bff_http_requests_in_flight` | — | Gauge |
| `bff_upstream_requests_total` | `upstream`, `grpc_status` | Counter |
| `bff_upstream_request_duration_seconds` | `upstream` | Histogram |

Expose metrics with `metrics_handler`:

```rust
use ddd_bff::metrics::metrics_handler;
app.route("/metrics", axum::routing::get(metrics_handler))
```

---

## Error mapping

`transcode::grpc_status_to_app_error` + `transcode::app_error_to_problem`
translate `tonic::Status` → `AppError` → RFC 9457 `application/problem+json`
with `urn:problem-type:*` URIs and `FieldViolation` arrays.

---

## Body redaction

`middleware::redaction::redact_json` recursively redacts configured field
names (case insensitive) from logged payloads. The redacted value is
`[REDACTED]`. Configure via `ObservabilityState::redact_fields`.

---

## Graceful shutdown

`edge::shutdown::wait_for_shutdown_signal()` returns a future that resolves on
SIGTERM or SIGINT. Wire it directly into axum:

```rust
axum::serve(listener, app)
    .with_graceful_shutdown(ddd_bff::edge::shutdown::wait_for_shutdown_signal())
    .await?;
```

---

## HTTP reverse proxy

`proxy::ProxyState` + `proxy::proxy_handler` forward every request under a
prefix to a downstream HTTP service, stripping the prefix before forwarding
and dropping hop-by-hop headers:

```rust
use ddd_bff::proxy::{ProxyState, proxy_handler};
use axum::routing::any;

let proxy = ProxyState::new(
    "http://order-service:8080".into(),
    "/admin/orders".into(),
    Duration::from_secs(5),
);

let app = Router::new()
    .route("/admin/orders/{*path}", any(proxy_handler))
    .with_state(proxy);
```

---

## Layout

```
src/
├── lib.rs
├── config.rs                # BffConfig, ResilienceConfig, env_or
├── metrics.rs               # BFF_METRICS singleton + metrics_handler
├── clients/
│   └── pool.rs              # GrpcClientPool, ResilientChannel
├── edge/
│   ├── route_config.rs      # routes.yaml parser
│   ├── path_template.rs     # {param} → matchit pattern compiler
│   ├── router.rs            # compiled route table
│   ├── upstream.rs          # UpstreamRegistry of pooled tonic channels
│   ├── service.rs           # BffEdge hyper Service
│   ├── observability.rs     # RequestObs + record_upstream
│   ├── shutdown.rs          # wait_for_shutdown_signal + drain helper
│   ├── server.rs            # hyper accept loop with cancellation
│   └── metrics_server.rs    # /metrics + /health hyper server
├── transcode/
│   ├── descriptors.rs       # install() / decode_pool() / load()
│   ├── codec.rs             # BytesCodec for tonic
│   ├── request.rs           # build protobuf from path/query/body
│   ├── response.rs          # protobuf → camelCase JSON
│   ├── streaming.rs         # gRPC stream → SSE
│   └── errors.rs            # ProblemDetail mapping
├── middleware/
│   ├── redaction.rs         # redact_json / redact_json_string
│   └── axum_observability.rs# ObservabilityState + observability_middleware (axum-response)
├── openapi/
│   ├── api_routes.rs        # ApiRoute, RouteKind, inject_routes
│   ├── router.rs            # openapi_router — Scalar UI + JSON spec (axum-response)
│   └── merge.rs             # merged_openapi — fetch + merge downstream spec (axum-response)
└── proxy.rs                 # ProxyState + proxy_handler (axum-response)
```

`proto/bff_aggregation.proto` + `build.rs` are kept as a **test fixture** so
the descriptor-pool tests remain self-contained — not part of the public API.

---

## Configuration

`BffConfig::from_env()` reads:

| Variable                | Default                              | Purpose                          |
|-------------------------|--------------------------------------|----------------------------------|
| `BFF_HOST`              | `0.0.0.0`                            | Edge bind host                   |
| `BFF_PORT`              | `3000`                               | Edge bind port                   |
| `BFF_METRICS_PORT`      | `9090`                               | Loopback `/metrics` port         |
| `BFF_ROUTES_PATH`       | `config/routes.yaml`                 | Route table (full-edge mode)     |
| `REQUEST_TIMEOUT_SECS`  | `30`                                 | Tower HTTP generic outer timeout |
| `GRPC_TIMEOUT_MS`       | `5000`                               | Per-call timeout                 |
| `GRPC_MAX_CONCURRENT`   | `100`                                | Per-channel concurrency cap      |
| `REDACT_FIELDS`         | `password,secret,token,authorization`| Logged-payload redaction list    |
| `OTLP_ENDPOINT`         | _unset_                              | Enables OTLP trace export        |
| `SHUTDOWN_TIMEOUT_SECS` | `30`                                 | Per-component drain budget       |

Service URLs are not part of `BffConfig`. Pass them to
`GrpcClientPool::from_services` directly or model them in a service-specific
config struct.

---

## Build / test

```
cd Src/crates/ddd-bff
cargo build --all-features
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
```
