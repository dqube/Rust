//! Compiled route table + dispatch.
//!
//! Takes a parsed [`RouteConfigFile`] plus a [`DescriptorPool`] and produces
//! a router that can answer `(method, path) → RouteMatch` at edge-request
//! time.
//!
//! Semantics:
//! - [`matchit`] handles literal-over-parameterised precedence and
//!   single-segment captures.
//! - Multiple routes that share a template (e.g. `GET /api/orders` and
//!   `POST /api/orders`) are stored in a per-template bucket; method
//!   filtering happens after path match.
//! - No path match → [`RouteMatch::NotFound`].
//! - Path match but no method match → [`RouteMatch::MethodNotAllowed`]
//!   with the set of allowed methods for the path.
//! - First route within a bucket whose methods include the request method
//!   wins (declaration order).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use ddd_shared_kernel::{AppError, AppResult};
use http::Method;
use prost_reflect::{DescriptorPool, MethodDescriptor};

use super::path_template::{self, to_matchit};
use super::route_config::{
    BindingSource, RouteConfigFile, RouteDef, TargetDef, UpstreamConfig,
};

/// A route that has been validated against the descriptor pool.
#[derive(Debug)]
pub struct CompiledRoute {
    /// Original definition as parsed from YAML.
    pub def: RouteDef,
    /// Parsed HTTP methods.
    pub allowed_methods: HashSet<Method>,
    /// Resolved gRPC method descriptor (present for unary / aggregation /
    /// streaming targets; `None` for static).
    pub method_descriptor: Option<MethodDescriptor>,
    /// Parameter names captured from the path template.
    pub path_params: Vec<String>,
}

/// Outcome of matching a `(method, path)` pair against the router.
#[derive(Debug)]
pub enum RouteMatch {
    /// A route matched.
    Matched {
        /// Compiled route.
        route: Arc<CompiledRoute>,
        /// Captured path parameters.
        params: HashMap<String, String>,
    },
    /// A route matched the path but not the method.
    MethodNotAllowed {
        /// Methods the matched path template accepts.
        allowed: Vec<Method>,
    },
    /// No route matched the path.
    NotFound,
}

/// Internal: group of routes sharing a path template.
#[derive(Debug)]
struct Bucket {
    routes: Vec<Arc<CompiledRoute>>,
}

/// Compiled route table.
pub struct Router {
    inner: matchit::Router<Arc<Bucket>>,
    upstreams: BTreeMap<String, UpstreamConfig>,
}

impl std::fmt::Debug for Router {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Router")
            .field("upstreams", &self.upstreams.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

impl Router {
    /// Build a router from parsed config, validating every gRPC target
    /// against the provided descriptor pool.
    pub fn from_config(cfg: RouteConfigFile, pool: &DescriptorPool) -> AppResult<Self> {
        let mut buckets: BTreeMap<String, Vec<Arc<CompiledRoute>>> = BTreeMap::new();

        for r in cfg.routes {
            let compiled = compile_route(&r, pool, &cfg.upstreams)?;
            buckets
                .entry(r.match_.path.clone())
                .or_default()
                .push(Arc::new(compiled));
        }

        let mut inner = matchit::Router::new();
        for (template, routes) in buckets {
            let matchit_path = to_matchit(&template);
            inner
                .insert(matchit_path.clone(), Arc::new(Bucket { routes }))
                .map_err(|e| AppError::Internal {
                    message: format!(
                        "failed to register route path `{template}` (matchit: `{matchit_path}`): {e}"
                    ),
                })?;
        }

        Ok(Self {
            inner,
            upstreams: cfg.upstreams,
        })
    }

    /// Look up a route by request method and path.
    pub fn match_request(&self, method: &Method, path: &str) -> RouteMatch {
        let matched = match self.inner.at(path) {
            Ok(m) => m,
            Err(_) => return RouteMatch::NotFound,
        };

        let bucket = Arc::clone(matched.value);
        let params: HashMap<String, String> = matched
            .params
            .iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();

        for route in &bucket.routes {
            if route.allowed_methods.contains(method) {
                return RouteMatch::Matched {
                    route: Arc::clone(route),
                    params,
                };
            }
        }

        let mut allowed: Vec<Method> = Vec::new();
        for route in &bucket.routes {
            for m in &route.allowed_methods {
                if !allowed.contains(m) {
                    allowed.push(m.clone());
                }
            }
        }
        RouteMatch::MethodNotAllowed { allowed }
    }

    /// Look up an upstream by name.
    pub fn upstream(&self, name: &str) -> Option<&UpstreamConfig> {
        self.upstreams.get(name)
    }
}

fn compile_route(
    r: &RouteDef,
    pool: &DescriptorPool,
    upstreams: &BTreeMap<String, UpstreamConfig>,
) -> AppResult<CompiledRoute> {
    let methods = parse_methods(&r.match_.methods)?;
    let path_params = path_template::extract_params(&r.match_.path);

    let method_descriptor = match &r.target {
        TargetDef::PassthroughUnary { upstream, grpc }
        | TargetDef::Aggregation { upstream, grpc }
        | TargetDef::PassthroughStream { upstream, grpc } => {
            if !upstreams.contains_key(upstream) {
                return Err(AppError::Internal {
                    message: format!(
                        "route `{}` references unknown upstream `{}`",
                        r.id, upstream
                    ),
                });
            }

            let service_desc = pool
                .get_service_by_name(&grpc.service)
                .ok_or_else(|| AppError::Internal {
                    message: format!(
                        "route `{}`: gRPC service `{}` not present in descriptor pool",
                        r.id, grpc.service
                    ),
                })?;

            let method_desc = service_desc
                .methods()
                .find(|m| m.name() == grpc.method)
                .ok_or_else(|| AppError::Internal {
                    message: format!(
                        "route `{}`: method `{}` not found on service `{}`",
                        r.id, grpc.method, grpc.service
                    ),
                })?;

            let input = method_desc.input();
            for b in &grpc.bindings {
                // Validate source parses
                let src = b.source()?;
                if let BindingSource::Path(ref name) = src {
                    if !path_params.contains(name) {
                        return Err(AppError::Internal {
                            message: format!(
                                "route `{}`: binding `path.{}` references a path \
                                 parameter that does not appear in `{}`",
                                r.id, name, r.match_.path
                            ),
                        });
                    }
                }
                // Validate destination field exists
                if input.get_field_by_name(&b.to).is_none() {
                    return Err(AppError::Internal {
                        message: format!(
                            "route `{}`: proto message `{}` has no field `{}`",
                            r.id,
                            input.full_name(),
                            b.to
                        ),
                    });
                }
            }

            if let Some(body_field) = &grpc.body {
                if body_field != "*" && input.get_field_by_name(body_field).is_none() {
                    return Err(AppError::Internal {
                        message: format!(
                            "route `{}`: `body: {}` references a field not present on `{}`",
                            r.id,
                            body_field,
                            input.full_name()
                        ),
                    });
                }
            }

            Some(method_desc)
        }
        TargetDef::Static { .. } => None,
    };

    Ok(CompiledRoute {
        def: r.clone(),
        allowed_methods: methods,
        method_descriptor,
        path_params,
    })
}

fn parse_methods(raw: &[String]) -> AppResult<HashSet<Method>> {
    let mut out = HashSet::new();
    for m in raw {
        let method = Method::from_str(m).map_err(|e| AppError::Serialization {
            message: format!("invalid HTTP method `{m}`: {e}"),
        })?;
        out.insert(method);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcode;

    const CFG: &str = r#"
upstreams:
  aggregator-core:
    endpoints: ["http://127.0.0.1:3100"]

routes:
  - id: get-aggregated-order
    match:
      path: /admin/orders/{order_id}/aggregated
      methods: [GET]
    target:
      kind: aggregation
      upstream: aggregator-core
      grpc:
        service: fixture.v1.FixtureService
        method: Echo
        bindings:
          - { from: path.order_id, to: message }

  - id: health
    match: { path: /health, methods: [GET] }
    target: { kind: static, status: 200, body: '{"status":"ok"}' }

  - id: ready
    match: { path: /ready, methods: [GET] }
    target: { kind: static, status: 200, body: '{"status":"ready"}' }
"#;

    fn router() -> Router {
        let cfg = RouteConfigFile::from_yaml(CFG).expect("yaml parses");
        let pool = transcode::load().expect("descriptor pool loads");
        Router::from_config(cfg, pool).expect("router builds")
    }

    #[test]
    fn matches_aggregation_with_path_param() {
        let r = router();
        match r.match_request(&Method::GET, "/admin/orders/abc-123/aggregated") {
            RouteMatch::Matched { route, params } => {
                assert_eq!(route.def.id, "get-aggregated-order");
                assert_eq!(params["order_id"], "abc-123");
                assert!(route.method_descriptor.is_some());
            }
            other => panic!("expected Matched, got {other:?}"),
        }
    }

    #[test]
    fn literal_path_match() {
        let r = router();
        match r.match_request(&Method::GET, "/health") {
            RouteMatch::Matched { route, params } => {
                assert_eq!(route.def.id, "health");
                assert!(params.is_empty());
                assert!(route.method_descriptor.is_none());
            }
            other => panic!("expected Matched, got {other:?}"),
        }
    }

    #[test]
    fn method_not_allowed() {
        let r = router();
        match r.match_request(&Method::POST, "/health") {
            RouteMatch::MethodNotAllowed { allowed } => {
                assert_eq!(allowed, vec![Method::GET]);
            }
            other => panic!("expected MethodNotAllowed, got {other:?}"),
        }
    }

    #[test]
    fn not_found() {
        let r = router();
        assert!(matches!(
            r.match_request(&Method::GET, "/does/not/exist"),
            RouteMatch::NotFound
        ));
    }

    #[test]
    fn rejects_unknown_upstream() {
        let cfg = RouteConfigFile::from_yaml(
            r#"
upstreams: {}
routes:
  - id: bad
    match: { path: /x, methods: [GET] }
    target:
      kind: passthrough_unary
      upstream: missing
      grpc:
        service: fixture.v1.FixtureService
        method: Echo
"#,
        )
        .unwrap();
        let pool = transcode::load().unwrap();
        let err = Router::from_config(cfg, pool).unwrap_err();
        assert!(format!("{err}").contains("unknown upstream"));
    }

    #[test]
    fn rejects_unknown_service() {
        let cfg = RouteConfigFile::from_yaml(
            r#"
upstreams:
  x: { endpoints: ["http://127.0.0.1:1"] }
routes:
  - id: bad
    match: { path: /x, methods: [GET] }
    target:
      kind: passthrough_unary
      upstream: x
      grpc:
        service: does.not.Exist
        method: Foo
"#,
        )
        .unwrap();
        let pool = transcode::load().unwrap();
        let err = Router::from_config(cfg, pool).unwrap_err();
        assert!(format!("{err}").contains("not present in descriptor pool"));
    }

    #[test]
    fn rejects_binding_to_missing_proto_field() {
        let cfg = RouteConfigFile::from_yaml(
            r#"
upstreams:
  x: { endpoints: ["http://127.0.0.1:1"] }
routes:
  - id: bad
    match:
      path: /x/{id}
      methods: [GET]
    target:
      kind: aggregation
      upstream: x
      grpc:
        service: fixture.v1.FixtureService
        method: Echo
        bindings:
          - { from: path.id, to: nonexistent_field }
"#,
        )
        .unwrap();
        let pool = transcode::load().unwrap();
        let err = Router::from_config(cfg, pool).unwrap_err();
        assert!(format!("{err}").contains("has no field"));
    }

    #[test]
    fn rejects_path_binding_missing_from_template() {
        let cfg = RouteConfigFile::from_yaml(
            r#"
upstreams:
  x: { endpoints: ["http://127.0.0.1:1"] }
routes:
  - id: bad
    match: { path: /x, methods: [GET] }
    target:
      kind: aggregation
      upstream: x
      grpc:
        service: fixture.v1.FixtureService
        method: Echo
        bindings:
          - { from: path.id, to: message }
"#,
        )
        .unwrap();
        let pool = transcode::load().unwrap();
        let err = Router::from_config(cfg, pool).unwrap_err();
        assert!(format!("{err}").contains("does not appear"));
    }

    #[test]
    fn literal_wins_over_param() {
        let cfg = RouteConfigFile::from_yaml(
            r#"
upstreams:
  x: { endpoints: ["http://127.0.0.1:1"] }
routes:
  - id: by-id
    match:
      path: /api/orders/{id}
      methods: [GET]
    target:
      kind: aggregation
      upstream: x
      grpc:
        service: fixture.v1.FixtureService
        method: Echo
        bindings:
          - { from: path.id, to: message }
  - id: export
    match: { path: /api/orders/export, methods: [GET] }
    target:
      kind: static
      status: 200
      body: ''
"#,
        )
        .unwrap();
        let pool = transcode::load().unwrap();
        let r = Router::from_config(cfg, pool).unwrap();
        match r.match_request(&Method::GET, "/api/orders/export") {
            RouteMatch::Matched { route, .. } => assert_eq!(route.def.id, "export"),
            other => panic!("literal should win: got {other:?}"),
        }
        match r.match_request(&Method::GET, "/api/orders/abc") {
            RouteMatch::Matched { route, params } => {
                assert_eq!(route.def.id, "by-id");
                assert_eq!(params["id"], "abc");
            }
            other => panic!("expected by-id match: got {other:?}"),
        }
    }

    #[test]
    fn multiple_methods_same_path() {
        let cfg = RouteConfigFile::from_yaml(
            r#"
upstreams:
  x: { endpoints: ["http://127.0.0.1:1"] }
routes:
  - id: list
    match: { path: /api/orders, methods: [GET] }
    target:
      kind: passthrough_unary
      upstream: x
      grpc:
        service: fixture.v1.FixtureService
        method: Echo
  - id: create
    match: { path: /api/orders, methods: [POST] }
    target:
      kind: passthrough_unary
      upstream: x
      grpc:
        service: fixture.v1.FixtureService
        method: Echo
        body: "*"
"#,
        )
        .unwrap();
        let pool = transcode::load().unwrap();
        let r = Router::from_config(cfg, pool).unwrap();

        match r.match_request(&Method::GET, "/api/orders") {
            RouteMatch::Matched { route, .. } => assert_eq!(route.def.id, "list"),
            other => panic!("{other:?}"),
        }
        match r.match_request(&Method::POST, "/api/orders") {
            RouteMatch::Matched { route, .. } => assert_eq!(route.def.id, "create"),
            other => panic!("{other:?}"),
        }
        match r.match_request(&Method::PUT, "/api/orders") {
            RouteMatch::MethodNotAllowed { allowed } => {
                assert!(allowed.contains(&Method::GET));
                assert!(allowed.contains(&Method::POST));
            }
            other => panic!("{other:?}"),
        }
    }
}
