# ddd-bff

Reusable building blocks for **Backend-for-Frontend** gateways.

## Standalone Examples

For full implementation details, see:
- [`grpc_client_pool.rs`](examples/grpc_client_pool.rs) — Configuring resilient gRPC client pools with timeouts and circuit breakers.
- [`openapi_catalogue.rs`](examples/openapi_catalogue.rs) — Defining declarative API route tables and injecting them into OpenAPI specs.

## What's in the box

| Module | What it gives you |
|---|---|
| `clients` | `GrpcClientPool` keyed by service name + `ResilientChannel` |
| `config` | Generic `BffConfig` (host, timeouts, resilience, redaction) |
| `edge` | Graceful shutdown signal handlers |
| `transcode` | gRPC `Status` → `AppError` → RFC 9457 `ProblemDetail` mapping |
| `metrics` | `BFF_METRICS` Prometheus singleton + `metrics_handler` |
| `middleware` | Axum observability, tracing, audit, body redaction, JWT auth |
| `openapi` | `ApiRoute`, `RouteKind`, `inject_routes`, Scalar router, downstream spec merge |
| `proxy` | `ProxyState` + `proxy_handler` — generic HTTP reverse proxy |

## Examples

### Resilient gRPC client pool

```rust
use ddd_bff::clients::GrpcClientPool;
use ddd_bff::config::ResilienceConfig;

let pool = GrpcClientPool::from_services(
    [("order", "http://order-service:50051")],
    &ResilienceConfig::default(),
)?;
let channel = pool.channel("order")?;
```

### HTTP reverse proxy

```rust
use ddd_bff::proxy::{ProxyState, proxy_handler};

let proxy = ProxyState::new(
    "http://legacy-service:8080".into(),
    "/admin/legacy".into(),
    Duration::from_secs(10),
);

let app = Router::new()
    .route("/admin/legacy/{*path}", any(proxy_handler))
    .with_state(proxy);
```

### OpenAPI catalogue

```rust
use ddd_bff::openapi::{inject_routes, ApiRoute, RouteKind};

const ROUTES: &[ApiRoute] = &[
    ApiRoute {
        kind: RouteKind::Passthrough {
            upstream: "order",
            grpc_method: "order.v1.OrderService/GetOrder",
        },
        method: "GET",
        path: "/orders/{id}",
        // ...
    },
];

inject_routes(&mut spec, ROUTES);
```

## Rules

- **Library only.** `ddd-bff` provides the pieces; consumers supply their own `main.rs` and wiring.
- No business logic. No per-service code generation.
- Error mapping translates `tonic::Status` → `AppError` → RFC 9457 `ProblemDetail`.
