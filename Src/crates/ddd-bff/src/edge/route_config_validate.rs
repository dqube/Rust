//! Descriptor-pool-independent validation for `routes.yaml`.
//!
//! Runs at bootstrap, before the descriptor-pool-aware
//! [`Router::from_config`] compile step. Reports every issue in a single
//! pass so operators see the full picture instead of fixing one typo, redeploying,
//! and hitting the next. Descriptor-level checks (service/method/field existence)
//! still live in `router.rs` since they require the proto pool.
//!
//! See [`crate::edge::route_config::RouteConfigFile`] for the types being
//! validated and [`ddd_shared_kernel::config_validation`] for the framework.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use ddd_shared_kernel::config_validation::{Report, Validate};
use http::Method;

use super::path_template;
use super::route_config::{RouteConfigFile, RouteDef, TargetDef, UpstreamConfig};

// ─── Impl ────────────────────────────────────────────────────────────────────

impl Validate for RouteConfigFile {
    fn validate(&self, report: &mut Report) {
        let upstream_names: HashSet<&str> = self.upstreams.keys().map(String::as_str).collect();
        let mut referenced: HashSet<String> = HashSet::new();
        let mut seen_ids: HashSet<&str> = HashSet::new();
        let mut seen_path_methods: HashMap<(String, Method), String> = HashMap::new();

        for (name, up) in &self.upstreams {
            validate_upstream(name, up, report);
        }

        for (idx, route) in self.routes.iter().enumerate() {
            validate_route(
                idx,
                route,
                report,
                &upstream_names,
                &mut referenced,
                &mut seen_ids,
                &mut seen_path_methods,
            );
        }

        for name in upstream_names {
            if !referenced.contains(name) {
                report.warn(
                    format!("upstreams.{name}"),
                    "unused_upstream",
                    format!("upstream `{name}` is declared but no route references it"),
                );
            }
        }
    }
}

// ─── Upstream checks ─────────────────────────────────────────────────────────

fn validate_upstream(name: &str, up: &UpstreamConfig, report: &mut Report) {
    let base = format!("upstreams.{name}");

    if name.is_empty() {
        report.error(&base, "empty_name", "upstream name must not be empty");
    }

    if up.endpoints.is_empty() {
        report.error(
            format!("{base}.endpoints"),
            "empty_endpoints",
            "upstream must declare at least one endpoint",
        );
    }

    for (i, ep) in up.endpoints.iter().enumerate() {
        let path = format!("{base}.endpoints[{i}]");
        validate_http_uri(&path, ep, report);
    }

    if up.timeout_ms == 0 {
        report.error(
            format!("{base}.timeout_ms"),
            "zero_timeout",
            "timeout_ms must be > 0",
        );
    }

    if up.max_concurrent == 0 {
        report.error(
            format!("{base}.max_concurrent"),
            "zero_concurrency",
            "max_concurrent must be > 0",
        );
    }
}

// ─── Route checks ────────────────────────────────────────────────────────────

fn validate_route<'a>(
    idx: usize,
    route: &'a RouteDef,
    report: &mut Report,
    upstream_names: &HashSet<&str>,
    referenced: &mut HashSet<String>,
    seen_ids: &mut HashSet<&'a str>,
    seen_path_methods: &mut HashMap<(String, Method), String>,
) {
    let base = format!("routes[{idx}]");

    // id
    if route.id.is_empty() {
        report.error(format!("{base}.id"), "empty_id", "route id must not be empty");
    } else if !seen_ids.insert(route.id.as_str()) {
        report.error(
            format!("{base}.id"),
            "duplicate_id",
            format!("duplicate route id `{}`", route.id),
        );
    }

    // path template
    if route.match_.path.is_empty() {
        report.error(
            format!("{base}.match.path"),
            "empty_path",
            "path template must not be empty",
        );
    } else if !route.match_.path.starts_with('/') {
        report.error(
            format!("{base}.match.path"),
            "path_not_rooted",
            format!("path `{}` must start with `/`", route.match_.path),
        );
    } else if unbalanced_braces(&route.match_.path) {
        report.error(
            format!("{base}.match.path"),
            "unbalanced_braces",
            format!("path `{}` has unbalanced `{{...}}`", route.match_.path),
        );
    }
    let path_params = path_template::extract_params(&route.match_.path);

    // methods
    let parsed_methods = validate_methods(&base, &route.match_.methods, report);

    // path+method collision
    for m in &parsed_methods {
        let key = (route.match_.path.clone(), m.clone());
        if let Some(prev) = seen_path_methods.get(&key) {
            report.error(
                format!("{base}.match"),
                "duplicate_path_method",
                format!(
                    "route `{}` conflicts with `{}`: both claim `{} {}`",
                    route.id, prev, m, route.match_.path
                ),
            );
        } else {
            seen_path_methods.insert(key, route.id.clone());
        }
    }

    // target
    validate_target(
        idx,
        route,
        report,
        upstream_names,
        referenced,
        &path_params,
    );
}

fn validate_methods(base: &str, raw: &[String], report: &mut Report) -> Vec<Method> {
    let mut out = Vec::new();
    if raw.is_empty() {
        report.error(
            format!("{base}.match.methods"),
            "empty_methods",
            "at least one HTTP method required",
        );
        return out;
    }
    let mut seen = HashSet::new();
    for (i, m) in raw.iter().enumerate() {
        match Method::from_str(m) {
            Ok(parsed) => {
                if !seen.insert(parsed.clone()) {
                    report.warn(
                        format!("{base}.match.methods[{i}]"),
                        "duplicate_method",
                        format!("method `{m}` listed more than once"),
                    );
                } else {
                    out.push(parsed);
                }
            }
            Err(_) => {
                report.error(
                    format!("{base}.match.methods[{i}]"),
                    "invalid_method",
                    format!("`{m}` is not a valid HTTP method"),
                );
            }
        }
    }
    out
}

fn validate_target(
    idx: usize,
    route: &RouteDef,
    report: &mut Report,
    upstream_names: &HashSet<&str>,
    referenced: &mut HashSet<String>,
    path_params: &[String],
) {
    let base = format!("routes[{idx}].target");
    match &route.target {
        TargetDef::PassthroughUnary { upstream, grpc }
        | TargetDef::Aggregation { upstream, grpc }
        | TargetDef::PassthroughStream { upstream, grpc } => {
            if upstream.is_empty() {
                report.error(
                    format!("{base}.upstream"),
                    "empty_upstream",
                    "target.upstream must not be empty",
                );
            } else {
                referenced.insert(upstream.clone());
                if !upstream_names.contains(upstream.as_str()) {
                    report.error(
                        format!("{base}.upstream"),
                        "unknown_upstream",
                        format!(
                            "route `{}` references upstream `{}` which is not declared",
                            route.id, upstream
                        ),
                    );
                }
            }

            if grpc.service.is_empty() {
                report.error(
                    format!("{base}.grpc.service"),
                    "empty_service",
                    "grpc.service must not be empty",
                );
            }
            if grpc.method.is_empty() {
                report.error(
                    format!("{base}.grpc.method"),
                    "empty_method",
                    "grpc.method must not be empty",
                );
            }

            if let Some(body) = &grpc.body {
                if body.is_empty() {
                    report.error(
                        format!("{base}.grpc.body"),
                        "empty_body",
                        "body must be `*` or a proto field name",
                    );
                }
            }

            for (bi, b) in grpc.bindings.iter().enumerate() {
                let bpath = format!("{base}.grpc.bindings[{bi}]");
                if b.to.is_empty() {
                    report.error(
                        format!("{bpath}.to"),
                        "empty_to",
                        "binding `to:` must name a proto field",
                    );
                }
                match b.source() {
                    Ok(src) => {
                        use super::route_config::BindingSource;
                        if let BindingSource::Path(name) = src {
                            if !path_params.iter().any(|p| p == &name) {
                                report.error(
                                    format!("{bpath}.from"),
                                    "missing_path_param",
                                    format!(
                                        "binding `path.{name}` does not appear in path template `{}`",
                                        route.match_.path
                                    ),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        report.error(
                            format!("{bpath}.from"),
                            "invalid_binding",
                            e.to_string(),
                        );
                    }
                }
            }
        }
        TargetDef::Static { status, .. } => {
            if !(100..=599).contains(status) {
                report.error(
                    format!("{base}.status"),
                    "invalid_status",
                    format!("status `{status}` is outside the HTTP 100-599 range"),
                );
            }
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn validate_http_uri(path: &str, raw: &str, report: &mut Report) {
    if raw.is_empty() {
        report.error(path, "empty_url", "URL must not be empty");
        return;
    }
    match http::Uri::from_str(raw) {
        Ok(uri) => match uri.scheme_str() {
            Some("http") | Some("https") => {}
            Some(other) => report.error(
                path,
                "unsupported_scheme",
                format!("URL `{raw}` uses unsupported scheme `{other}` (need http or https)"),
            ),
            None => report.error(
                path,
                "missing_scheme",
                format!("URL `{raw}` is missing an `http(s)://` scheme"),
            ),
        },
        Err(e) => report.error(
            path,
            "invalid_url",
            format!("URL `{raw}` is malformed: {e}"),
        ),
    }
}

fn unbalanced_braces(s: &str) -> bool {
    let mut depth: i32 = 0;
    for c in s.chars() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth < 0 {
                    return true;
                }
            }
            _ => {}
        }
    }
    depth != 0
}

// ─── Loader ──────────────────────────────────────────────────────────────────

impl RouteConfigFile {
    /// Read a YAML file, deserialize it, and run sanity-check validation in
    /// one step.
    ///
    /// On success returns the parsed config. On failure returns
    /// [`AppError::ValidationBatch`] with every detected issue. Warnings
    /// (e.g. unused upstreams) are logged via `tracing` but never fail.
    pub fn load_validated(
        path: impl AsRef<std::path::Path>,
    ) -> ddd_shared_kernel::AppResult<Self> {
        ddd_shared_kernel::config_validation::load_yaml_validated(path)
    }

    /// Parse YAML source and run sanity-check validation. Same semantics as
    /// [`load_validated`] but without touching the filesystem.
    pub fn from_yaml_validated(
        src: &str,
        context: impl Into<String>,
    ) -> ddd_shared_kernel::AppResult<Self> {
        ddd_shared_kernel::config_validation::from_yaml_validated(src, context)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ddd_shared_kernel::AppError;

    const VALID: &str = r#"
upstreams:
  order-svc:
    endpoints: ["http://127.0.0.1:50051"]
    timeout_ms: 4000
routes:
  - id: get-order
    match: { path: "/api/orders/{id}", methods: [GET] }
    target:
      kind: passthrough_unary
      upstream: order-svc
      grpc:
        service: order.v1.OrderService
        method: GetOrder
        bindings: [{ from: path.id, to: id }]
"#;

    fn parse(src: &str) -> RouteConfigFile {
        serde_yaml::from_str(src).unwrap()
    }

    #[test]
    fn valid_config_has_no_errors() {
        let cfg = parse(VALID);
        let report = cfg.build_report();
        assert!(report.is_ok(), "errors: {:?}", report.errors());
    }

    #[test]
    fn detects_duplicate_route_ids() {
        let cfg = parse(
            r#"
upstreams:
  u: { endpoints: ["http://127.0.0.1:50051"] }
routes:
  - id: dup
    match: { path: /a, methods: [GET] }
    target: { kind: static, status: 200, body: "" }
  - id: dup
    match: { path: /b, methods: [GET] }
    target: { kind: static, status: 200, body: "" }
"#,
        );
        let r = cfg.build_report();
        assert!(r.errors().iter().any(|e| e.code == "duplicate_id"));
    }

    #[test]
    fn detects_unknown_upstream() {
        let cfg = parse(
            r#"
upstreams:
  known: { endpoints: ["http://127.0.0.1:50051"] }
routes:
  - id: r
    match: { path: "/a/{id}", methods: [GET] }
    target:
      kind: passthrough_unary
      upstream: missing
      grpc: { service: s.v1.S, method: M, bindings: [{ from: path.id, to: id }] }
"#,
        );
        let r = cfg.build_report();
        assert!(r.errors().iter().any(|e| e.code == "unknown_upstream"));
    }

    #[test]
    fn detects_unused_upstream_as_warning() {
        let cfg = parse(
            r#"
upstreams:
  used:   { endpoints: ["http://127.0.0.1:50051"] }
  unused: { endpoints: ["http://127.0.0.1:50052"] }
routes:
  - id: r
    match: { path: "/a/{id}", methods: [GET] }
    target:
      kind: passthrough_unary
      upstream: used
      grpc: { service: s.v1.S, method: M, bindings: [{ from: path.id, to: id }] }
"#,
        );
        let r = cfg.build_report();
        assert!(r.is_ok()); // warnings don't fail
        assert!(r.warnings().iter().any(|w| w.code == "unused_upstream"));
    }

    #[test]
    fn detects_duplicate_path_method() {
        let cfg = parse(
            r#"
upstreams:
  u: { endpoints: ["http://127.0.0.1:50051"] }
routes:
  - id: a
    match: { path: /same, methods: [GET, POST] }
    target: { kind: static, status: 200, body: "" }
  - id: b
    match: { path: /same, methods: [GET] }
    target: { kind: static, status: 200, body: "" }
"#,
        );
        let r = cfg.build_report();
        assert!(r.errors().iter().any(|e| e.code == "duplicate_path_method"));
    }

    #[test]
    fn detects_invalid_url() {
        let cfg = parse(
            r#"
upstreams:
  u: { endpoints: ["ftp://bad"] }
routes: []
"#,
        );
        let r = cfg.build_report();
        assert!(r.errors().iter().any(|e| e.code == "unsupported_scheme"));
    }

    #[test]
    fn detects_zero_timeout() {
        let cfg = parse(
            r#"
upstreams:
  u: { endpoints: ["http://localhost:50051"], timeout_ms: 0 }
routes: []
"#,
        );
        let r = cfg.build_report();
        assert!(r.errors().iter().any(|e| e.code == "zero_timeout"));
    }

    #[test]
    fn detects_missing_path_param_in_binding() {
        let cfg = parse(
            r#"
upstreams:
  u: { endpoints: ["http://127.0.0.1:50051"] }
routes:
  - id: r
    match: { path: /orders, methods: [GET] }
    target:
      kind: passthrough_unary
      upstream: u
      grpc: { service: s.v1.S, method: M, bindings: [{ from: path.id, to: id }] }
"#,
        );
        let r = cfg.build_report();
        assert!(r.errors().iter().any(|e| e.code == "missing_path_param"));
    }

    #[test]
    fn detects_invalid_method() {
        let cfg = parse(
            r#"
upstreams:
  u: { endpoints: ["http://127.0.0.1:50051"] }
routes:
  - id: r
    match: { path: /x, methods: ["INVALID METHOD"] }
    target: { kind: static, status: 200, body: "" }
"#,
        );
        let r = cfg.build_report();
        assert!(r.errors().iter().any(|e| e.code == "invalid_method"));
    }

    #[test]
    fn detects_path_not_rooted() {
        let cfg = parse(
            r#"
upstreams:
  u: { endpoints: ["http://127.0.0.1:50051"] }
routes:
  - id: r
    match: { path: "no-slash", methods: [GET] }
    target: { kind: static, status: 200, body: "" }
"#,
        );
        let r = cfg.build_report();
        assert!(r.errors().iter().any(|e| e.code == "path_not_rooted"));
    }

    #[test]
    fn from_yaml_validated_bubbles_up_as_app_error() {
        let src = r#"
upstreams:
  u: { endpoints: ["http://127.0.0.1:50051"], timeout_ms: 0 }
routes: []
"#;
        let err = RouteConfigFile::from_yaml_validated(src, "test").unwrap_err();
        match err {
            AppError::ValidationBatch { errors } => {
                assert!(errors.iter().any(|e| e.code == "zero_timeout"));
            }
            other => panic!("expected ValidationBatch, got {other:?}"),
        }
    }

    #[test]
    fn collects_multiple_issues_in_one_pass() {
        let cfg = parse(
            r#"
upstreams:
  u: { endpoints: [], timeout_ms: 0 }
routes:
  - id: ""
    match: { path: "bad", methods: [] }
    target: { kind: static, status: 999, body: "" }
"#,
        );
        let r = cfg.build_report();
        // Multiple distinct issues should surface together.
        assert!(r.errors().len() >= 5, "got {} errors: {:?}", r.errors().len(), r.errors());
    }
}
