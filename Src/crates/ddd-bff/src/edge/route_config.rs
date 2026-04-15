//! Serde types for `routes.yaml`.
//!
//! The schema is described in `pingora-routing.md`. Summary:
//!
//! ```yaml
//! upstreams:
//!   order-svc: { endpoints: ["http://127.0.0.1:50051"], timeout_ms: 5000 }
//! routes:
//!   - id: get-order
//!     match:
//!       path: /api/orders/{id}
//!       methods: [GET]
//!     target:
//!       kind: passthrough_unary
//!       upstream: order-svc
//!       grpc:
//!         service: order.v1.OrderService
//!         method: GetOrder
//!         bindings:
//!           - from: path.id
//!             to:   id
//! ```

use std::collections::BTreeMap;

use ddd_shared_kernel::{AppError, AppResult};
use serde::{Deserialize, Serialize};

/// Top-level deserialized route configuration.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RouteConfigFile {
    /// Map of upstream name → config.
    #[serde(default)]
    pub upstreams: BTreeMap<String, UpstreamConfig>,

    /// Ordered route list. Declaration order is the tiebreaker for equal
    /// specificity.
    #[serde(default)]
    pub routes: Vec<RouteDef>,
}

impl RouteConfigFile {
    /// Parse a YAML string into a [`RouteConfigFile`].
    pub fn from_yaml(src: &str) -> AppResult<Self> {
        serde_yaml::from_str(src).map_err(|e| AppError::Serialization {
            message: format!("invalid routes.yaml: {e}"),
        })
    }
}

/// Configuration for a single gRPC upstream target.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpstreamConfig {
    /// gRPC endpoint URIs (e.g. `http://127.0.0.1:50051`). When multiple,
    /// load-balancing is delegated to pingora.
    pub endpoints: Vec<String>,

    /// Per-request timeout in milliseconds.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,

    /// Maximum in-flight concurrent requests to this upstream.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
}

fn default_timeout_ms() -> u64 {
    5_000
}

fn default_max_concurrent() -> usize {
    100
}

/// A single REST route with its match predicate and target.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RouteDef {
    /// Stable identifier used for metrics and logs.
    pub id: String,

    /// Match predicate (path template + allowed methods).
    #[serde(rename = "match")]
    pub match_: MatchDef,

    /// Target upstream / gRPC method / static response.
    pub target: TargetDef,

    /// Named middleware to run in order (auth, rate limit, …). Interpreted
    /// by the edge service — validated here only by presence.
    #[serde(default)]
    pub middleware: Vec<serde_yaml::Value>,
}

/// Match predicate for a route.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MatchDef {
    /// Path template using `{name}` for a single-segment capture and
    /// `{**name}` for a catch-all.
    pub path: String,

    /// HTTP methods this route accepts. Defaults to `[GET]`.
    #[serde(default = "default_methods")]
    pub methods: Vec<String>,
}

fn default_methods() -> Vec<String> {
    vec!["GET".to_owned()]
}

/// What the route forwards to.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TargetDef {
    /// Unary REST → gRPC unary passthrough.
    PassthroughUnary {
        /// Upstream name (must exist in `upstreams`).
        upstream: String,
        /// gRPC method descriptor coordinates.
        grpc: GrpcTarget,
    },
    /// REST → internal gRPC aggregator — identical transcode path as
    /// `passthrough_unary` but pointing at the in-process aggregator.
    Aggregation {
        /// Upstream name.
        upstream: String,
        /// gRPC method descriptor coordinates.
        grpc: GrpcTarget,
    },
    /// REST (SSE) → gRPC server-streaming.
    PassthroughStream {
        /// Upstream name.
        upstream: String,
        /// gRPC method descriptor coordinates.
        grpc: GrpcTarget,
    },
    /// Static inline response — used for `/health` and `/ready`.
    Static {
        /// HTTP status code to return.
        #[serde(default = "default_static_status")]
        status: u16,
        /// Response body.
        #[serde(default)]
        body: String,
    },
}

fn default_static_status() -> u16 {
    200
}

/// gRPC method coordinates for a route.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GrpcTarget {
    /// Fully qualified service name, e.g. `order.v1.OrderService`.
    pub service: String,
    /// Method name on the service.
    pub method: String,
    /// REST → proto field bindings.
    #[serde(default)]
    pub bindings: Vec<Binding>,
    /// Body binding. Either `"*"` (merge entire JSON body into the proto
    /// root) or a proto message field name (deserialize JSON body into
    /// that field).
    #[serde(default)]
    pub body: Option<String>,
}

/// A single REST → proto field binding.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Binding {
    /// Source: `path.<name>`, `query.<name>`, `header.<name>`, or `body`.
    pub from: String,
    /// Destination proto field name on the request message.
    pub to: String,
}

/// Parsed binding source after splitting the `from:` string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingSource {
    /// A captured path segment.
    Path(String),
    /// A URL query string key.
    Query(String),
    /// An HTTP request header.
    Header(String),
    /// The entire request body.
    Body,
}

impl Binding {
    /// Parse the `from:` field into a structured [`BindingSource`].
    pub fn source(&self) -> AppResult<BindingSource> {
        if self.from == "body" {
            return Ok(BindingSource::Body);
        }
        let (kind, name) =
            self.from
                .split_once('.')
                .ok_or_else(|| AppError::Serialization {
                    message: format!(
                        "binding `from: {}` must be `body`, `path.X`, `query.X`, or `header.X`",
                        self.from
                    ),
                })?;
        match kind {
            "path" => Ok(BindingSource::Path(name.to_owned())),
            "query" => Ok(BindingSource::Query(name.to_owned())),
            "header" => Ok(BindingSource::Header(name.to_owned())),
            other => Err(AppError::Serialization {
                message: format!(
                    "binding source `{other}` not recognised (expected body, path, query, header)"
                ),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
upstreams:
  order-svc:
    endpoints: ["http://127.0.0.1:50051"]
    timeout_ms: 4000
  aggregator-core:
    endpoints: ["http://127.0.0.1:3100"]

routes:
  - id: get-order
    match:
      path: /api/orders/{id}
      methods: [GET]
    target:
      kind: passthrough_unary
      upstream: order-svc
      grpc:
        service: order.v1.OrderService
        method: GetOrder
        bindings:
          - { from: path.id, to: id }
  - id: create-order
    match:
      path: /api/orders
      methods: [POST]
    target:
      kind: passthrough_unary
      upstream: order-svc
      grpc:
        service: order.v1.OrderService
        method: CreateOrder
        body: "*"
  - id: health
    match: { path: /health, methods: [GET] }
    target: { kind: static, status: 200, body: '{"status":"ok"}' }
"#;

    #[test]
    fn parses_sample_config() {
        let cfg = RouteConfigFile::from_yaml(SAMPLE).expect("parses");
        assert_eq!(cfg.upstreams.len(), 2);
        assert_eq!(cfg.upstreams["order-svc"].timeout_ms, 4000);
        assert_eq!(cfg.upstreams["aggregator-core"].timeout_ms, 5000); // default
        assert_eq!(cfg.routes.len(), 3);
    }

    #[test]
    fn binding_source_parses() {
        let b = Binding {
            from: "path.id".to_owned(),
            to: "id".to_owned(),
        };
        assert_eq!(b.source().unwrap(), BindingSource::Path("id".to_owned()));

        let b = Binding {
            from: "body".to_owned(),
            to: "x".to_owned(),
        };
        assert_eq!(b.source().unwrap(), BindingSource::Body);

        let b = Binding {
            from: "header.X-Request-Id".to_owned(),
            to: "request_id".to_owned(),
        };
        assert_eq!(
            b.source().unwrap(),
            BindingSource::Header("X-Request-Id".to_owned())
        );

        let b = Binding {
            from: "bogus".to_owned(),
            to: "x".to_owned(),
        };
        assert!(b.source().is_err());
    }
}
