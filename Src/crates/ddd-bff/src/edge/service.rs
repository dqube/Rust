//! HTTP edge dispatcher.
//!
//! Hyper-based `Service` that receives REST requests, runs them through the
//! router, transcodes REST ↔ gRPC, and dispatches to the matched upstream
//! through a byte-passthrough tonic client. Streaming responses (SSE) are
//! produced in a later module; this file currently handles unary traffic
//! plus static routes.

use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use ddd_shared_kernel::AppError;
use http::{HeaderMap, Method, Request, Response, StatusCode};
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::Service;
use prost_reflect::MethodDescriptor;
use tonic::client::Grpc;
use tonic::transport::Channel;

use super::observability::{record_upstream, RequestObs, UNKNOWN_ROUTE};
use super::route_config::TargetDef;
use super::router::{RouteMatch, Router};
use super::upstream::UpstreamRegistry;
use crate::transcode;
use crate::transcode::errors::{
    app_error_to_problem, grpc_status_to_app_error, method_not_allowed, route_not_found,
    upstream_unavailable, ProblemDetail, PROBLEM_CONTENT_TYPE,
};
use crate::transcode::request::TranscodeRequest;

/// Shared state for the edge service. Cheaply cloneable.
#[derive(Clone)]
pub struct EdgeState {
    /// Compiled route table.
    pub router: Arc<Router>,
    /// Registry of upstream gRPC channels.
    pub upstreams: UpstreamRegistry,
}

impl EdgeState {
    /// Bundle router + upstreams into a shareable state.
    pub fn new(router: Router, upstreams: UpstreamRegistry) -> Self {
        Self {
            router: Arc::new(router),
            upstreams,
        }
    }

    /// Bootstrap edge state from a `routes.yaml` file, running the full
    /// three-stage validation pipeline before returning.
    ///
    /// ## Stages
    ///
    /// 1. **Schema + sanity checks** (`Validate` impl on `RouteConfigFile`)
    ///    — missing / duplicate upstream declarations, unknown upstream
    ///    references, bad HTTP method names, unbalanced path-template braces,
    ///    path+method collisions, zero timeouts, and malformed endpoint URLs.
    ///    All errors are collected in a single pass and returned together as
    ///    `AppError::ValidationBatch` so operators can fix everything at once.
    ///
    /// 2. **Descriptor-level checks** (`Router::from_config`)
    ///    — confirms every `grpc.service` and `grpc.method` name exists in
    ///    the embedded proto descriptor pool and that every `bindings[].to`
    ///    field name exists on the proto message.
    ///
    /// 3. **Channel construction** (`UpstreamRegistry::from_config`)
    ///    — builds lazy gRPC channels; aborts with an error if a declared
    ///    endpoint URI cannot be parsed by tonic.
    ///
    /// ## Intended use
    ///
    /// Call this once during `main` before binding the server port.  Any
    /// error should be treated as fatal:
    ///
    /// ```no_run
    /// let state = ddd_bff::edge::EdgeState::from_routes_file("config/routes.yaml")
    ///     .expect("invalid routes.yaml — cannot start");
    /// ```
    pub fn from_routes_file(
        path: impl AsRef<std::path::Path>,
    ) -> ddd_shared_kernel::AppResult<Self> {
        use super::route_config::RouteConfigFile;

        // Stage 1: load the YAML file and run schema + sanity validation.
        let cfg = RouteConfigFile::load_validated(path.as_ref())?;

        // Stage 2: descriptor-level validation against the embedded proto pool.
        let pool = transcode::load()?;
        let upstreams_cfg = cfg.upstreams.clone();
        let router = Router::from_config(cfg, pool)?;

        // Stage 3: build lazy upstream gRPC channels.
        let upstreams = UpstreamRegistry::from_config(&upstreams_cfg)?;

        Ok(Self::new(router, upstreams))
    }
}

/// Hyper [`Service`] implementing the BFF edge.
#[derive(Clone)]
pub struct BffEdge {
    state: EdgeState,
}

impl BffEdge {
    /// Build an edge service from shared state.
    pub fn new(state: EdgeState) -> Self {
        Self { state }
    }
}

/// Response body type used throughout the edge. Boxing keeps the service
/// polymorphic over unary JSON and SSE streaming futures. Unsync because
/// tonic's [`tonic::Streaming`] is not `Sync`.
pub type BodyT = UnsyncBoxBody<Bytes, std::io::Error>;

impl Service<Request<Incoming>> for BffEdge {
    type Response = Response<BodyT>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let state = self.state.clone();
        Box::pin(async move { Ok(dispatch(state, req).await) })
    }
}

/// Entry point called by both hyper and tests. Wraps the inner dispatch
/// in per-request observability (request-id, in-flight gauge, request
/// count + duration metrics, structured access log).
pub async fn dispatch(state: EdgeState, req: Request<Incoming>) -> Response<BodyT> {
    let (mut parts, body) = req.into_parts();

    let obs = RequestObs::start(&parts.method, parts.uri.path(), &parts.headers);
    // Make the correlation id visible to dispatch_unary/stream so it gets
    // forwarded to downstream gRPC calls.
    obs.ensure_in_headers(&mut parts.headers);

    let (mut response, route_id) = dispatch_inner(state, parts, body).await;
    let status = response.status().as_u16();
    obs.stamp_response(&mut response);
    obs.finish(route_id.as_deref().unwrap_or(UNKNOWN_ROUTE), status);
    response
}

async fn dispatch_inner(
    state: EdgeState,
    parts: http::request::Parts,
    body: Incoming,
) -> (Response<BodyT>, Option<String>) {
    let outcome = state
        .router
        .match_request(&parts.method, parts.uri.path());

    match outcome {
        RouteMatch::NotFound => {
            let pd = route_not_found(parts.method.as_str(), parts.uri.path());
            (problem_response(pd), None)
        }
        RouteMatch::MethodNotAllowed { allowed } => {
            let pd = method_not_allowed(
                parts.method.as_str(),
                &allowed.iter().map(|m| m.as_str().to_owned()).collect::<Vec<_>>(),
            );
            let mut resp = problem_response(pd);
            let allow = allowed
                .iter()
                .map(|m| m.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            if let Ok(v) = http::HeaderValue::from_str(&allow) {
                resp.headers_mut().insert(http::header::ALLOW, v);
            }
            (resp, None)
        }
        RouteMatch::Matched { route, params } => {
            let route_id = route.def.id.clone();
            let resp = match &route.def.target {
                TargetDef::Static { status, body } => static_response(*status, body.clone()),
                TargetDef::PassthroughUnary { upstream, grpc }
                | TargetDef::Aggregation { upstream, grpc } => {
                    let method_desc = match route.method_descriptor.clone() {
                        Some(md) => md,
                        None => {
                            return (
                                problem_response(app_error_to_problem(&AppError::internal(
                                    "method descriptor missing on matched route",
                                ))),
                                Some(route_id),
                            );
                        }
                    };
                    let upstream_name = upstream.clone();
                    let upstream_entry = match state.upstreams.get(&upstream_name) {
                        Some(up) => up.clone(),
                        None => {
                            return (
                                problem_response(app_error_to_problem(&AppError::internal(
                                    format!("unknown upstream: {upstream_name}"),
                                ))),
                                Some(route_id),
                            );
                        }
                    };

                    let query = transcode::parse_query(parts.uri.query());
                    let body_bytes = match collect_body(body).await {
                        Ok(b) => b,
                        Err(pd) => return (problem_response(pd), Some(route_id)),
                    };

                    dispatch_unary(DispatchUnary {
                        method_desc,
                        grpc: grpc.clone(),
                        channel: upstream_entry.channel,
                        upstream_name,
                        method: parts.method,
                        headers: parts.headers,
                        path_params: params,
                        query,
                        body: body_bytes,
                    })
                    .await
                }
                TargetDef::PassthroughStream { upstream, grpc } => {
                    let method_desc = match route.method_descriptor.clone() {
                        Some(md) => md,
                        None => {
                            return (
                                problem_response(app_error_to_problem(&AppError::internal(
                                    "method descriptor missing on matched route",
                                ))),
                                Some(route_id),
                            );
                        }
                    };
                    let upstream_name = upstream.clone();
                    let upstream_entry = match state.upstreams.get(&upstream_name) {
                        Some(up) => up.clone(),
                        None => {
                            return (
                                problem_response(app_error_to_problem(&AppError::internal(
                                    format!("unknown upstream: {upstream_name}"),
                                ))),
                                Some(route_id),
                            );
                        }
                    };

                    let query = transcode::parse_query(parts.uri.query());
                    let body_bytes = match collect_body(body).await {
                        Ok(b) => b,
                        Err(pd) => return (problem_response(pd), Some(route_id)),
                    };

                    dispatch_stream(DispatchStream {
                        method_desc,
                        grpc: grpc.clone(),
                        channel: upstream_entry.channel,
                        upstream_name,
                        headers: parts.headers,
                        path_params: params,
                        query,
                        body: body_bytes,
                    })
                    .await
                }
            };
            (resp, Some(route_id))
        }
    }
}

struct DispatchUnary {
    method_desc: MethodDescriptor,
    grpc: super::route_config::GrpcTarget,
    channel: Channel,
    upstream_name: String,
    method: Method,
    headers: HeaderMap,
    path_params: HashMap<String, String>,
    query: HashMap<String, String>,
    body: Bytes,
}

async fn dispatch_unary(inputs: DispatchUnary) -> Response<BodyT> {
    let DispatchUnary {
        method_desc,
        grpc,
        channel,
        upstream_name,
        method,
        headers,
        path_params,
        query,
        body,
    } = inputs;

    let encoded = match transcode::encode_request(TranscodeRequest {
        method_desc: &method_desc,
        grpc: &grpc,
        path_params: &path_params,
        query: &query,
        headers: &headers,
        body: &body,
    }) {
        Ok(e) => e,
        Err(err) => return problem_response(app_error_to_problem(&err)),
    };

    // Tonic 0.10 uses `http` 0.2 internally; our edge uses `http` 1.x. Bridge
    // via tonic's re-exported http module to avoid type mismatches.
    let path = match tonic::codegen::http::uri::PathAndQuery::try_from(encoded.grpc_path.as_str()) {
        Ok(p) => p,
        Err(e) => {
            return problem_response(app_error_to_problem(&AppError::internal(format!(
                "invalid grpc path `{}`: {e}",
                encoded.grpc_path
            ))));
        }
    };

    let mut request = tonic::Request::new(encoded.proto_bytes);

    // Propagate correlation headers so downstream services can log/trace.
    for header_name in ["x-request-id", "traceparent", "tracestate", "authorization"] {
        if let Some(value) = headers.get(header_name) {
            if let Ok(v) = value.to_str() {
                if let Ok(md) = v.parse() {
                    request.metadata_mut().insert(header_name, md);
                }
            }
        }
    }

    let mut grpc_client = Grpc::new(channel);
    let upstream_start = Instant::now();
    let tonic_response = match grpc_client
        .unary::<Bytes, Bytes, _>(request, path, transcode::BytesCodec)
        .await
    {
        Ok(r) => {
            record_upstream(
                &upstream_name,
                tonic::Code::Ok as i32,
                upstream_start.elapsed().as_secs_f64(),
            );
            r
        }
        Err(status) => {
            record_upstream(
                &upstream_name,
                status.code() as i32,
                upstream_start.elapsed().as_secs_f64(),
            );
            // Distinguish transport failures (502) from gRPC-level errors
            // (mapped by their code).
            if status.code() == tonic::Code::Unknown && status.message().contains("transport error")
            {
                return problem_response(upstream_unavailable(status.message().to_owned()));
            }
            let err = grpc_status_to_app_error(status);
            return problem_response(app_error_to_problem(&err));
        }
    };

    let proto_bytes = tonic_response.into_inner();
    let success_status = default_success_status(&method);
    let transcoded =
        transcode::transcode_unframed(&method_desc, &proto_bytes, success_status);

    match transcoded {
        transcode::TranscodedResponse::Json { body, status } => json_response(status, body),
        transcode::TranscodedResponse::Problem(pd) => problem_response(pd),
    }
}

fn default_success_status(method: &Method) -> u16 {
    match *method {
        Method::POST => 201,
        _ => 200,
    }
}

struct DispatchStream {
    method_desc: MethodDescriptor,
    grpc: super::route_config::GrpcTarget,
    channel: Channel,
    upstream_name: String,
    headers: HeaderMap,
    path_params: HashMap<String, String>,
    query: HashMap<String, String>,
    body: Bytes,
}

async fn dispatch_stream(inputs: DispatchStream) -> Response<BodyT> {
    let DispatchStream {
        method_desc,
        grpc,
        channel,
        upstream_name,
        headers,
        path_params,
        query,
        body,
    } = inputs;

    // Encode REST → unframed proto bytes (tonic frames internally).
    let encoded = match transcode::encode_request(TranscodeRequest {
        method_desc: &method_desc,
        grpc: &grpc,
        path_params: &path_params,
        query: &query,
        headers: &headers,
        body: &body,
    }) {
        Ok(e) => e,
        Err(err) => return problem_response(app_error_to_problem(&err)),
    };

    let path = match tonic::codegen::http::uri::PathAndQuery::try_from(encoded.grpc_path.as_str()) {
        Ok(p) => p,
        Err(e) => {
            return problem_response(app_error_to_problem(&AppError::internal(format!(
                "invalid grpc path `{}`: {e}",
                encoded.grpc_path
            ))));
        }
    };

    let mut request = tonic::Request::new(encoded.proto_bytes);
    for header_name in ["x-request-id", "traceparent", "tracestate", "authorization"] {
        if let Some(value) = headers.get(header_name) {
            if let Ok(v) = value.to_str() {
                if let Ok(md) = v.parse() {
                    request.metadata_mut().insert(header_name, md);
                }
            }
        }
    }

    let mut grpc_client = Grpc::new(channel);
    let upstream_start = Instant::now();
    let tonic_response = match grpc_client
        .server_streaming::<Bytes, Bytes, _>(request, path, transcode::BytesCodec)
        .await
    {
        Ok(r) => {
            // Record the call-establishment outcome. Per-message timing on
            // a server-stream is harder to attribute; the call counter is
            // recorded once when the stream opens.
            record_upstream(
                &upstream_name,
                tonic::Code::Ok as i32,
                upstream_start.elapsed().as_secs_f64(),
            );
            r
        }
        Err(status) => {
            record_upstream(
                &upstream_name,
                status.code() as i32,
                upstream_start.elapsed().as_secs_f64(),
            );
            if status.code() == tonic::Code::Unknown && status.message().contains("transport error")
            {
                return problem_response(upstream_unavailable(status.message().to_owned()));
            }
            let err = grpc_status_to_app_error(status);
            return problem_response(app_error_to_problem(&err));
        }
    };

    let upstream_stream = tonic_response.into_inner();
    let sse = transcode::into_sse_stream(
        upstream_stream,
        method_desc,
        transcode::DEFAULT_KEEPALIVE,
    );
    sse_response(transcode::sse_body(sse))
}

fn sse_response(body: BodyT) -> Response<BodyT> {
    let mut resp = Response::new(body);
    *resp.status_mut() = StatusCode::OK;
    let h = resp.headers_mut();
    h.insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static(transcode::SSE_CONTENT_TYPE),
    );
    h.insert(
        http::header::CACHE_CONTROL,
        http::HeaderValue::from_static("no-cache"),
    );
    h.insert(
        http::header::CONNECTION,
        http::HeaderValue::from_static("keep-alive"),
    );
    resp
}

async fn collect_body(body: Incoming) -> Result<Bytes, ProblemDetail> {
    match body.collect().await {
        Ok(collected) => Ok(collected.to_bytes()),
        Err(e) => Err(app_error_to_problem(&AppError::internal(format!(
            "failed to read request body: {e}"
        )))),
    }
}

/// Build a response that streams `Full<Bytes>` content.
fn full_body(bytes: impl Into<Bytes>) -> BodyT {
    Full::new(bytes.into())
        .map_err(|never| match never {})
        .boxed_unsync()
}

fn json_response(status: u16, body: Vec<u8>) -> Response<BodyT> {
    let mut resp = Response::new(full_body(Bytes::from(body)));
    *resp.status_mut() = StatusCode::from_u16(status).unwrap_or(StatusCode::OK);
    resp.headers_mut().insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("application/json"),
    );
    resp
}

fn problem_response(pd: ProblemDetail) -> Response<BodyT> {
    let status = StatusCode::from_u16(pd.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let body = pd.to_body();
    let mut resp = Response::new(full_body(Bytes::from(body)));
    *resp.status_mut() = status;
    resp.headers_mut().insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static(PROBLEM_CONTENT_TYPE),
    );
    resp
}

fn static_response(status: u16, body: String) -> Response<BodyT> {
    let mut resp = Response::new(full_body(Bytes::from(body)));
    *resp.status_mut() = StatusCode::from_u16(status).unwrap_or(StatusCode::OK);
    resp.headers_mut().insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("application/json"),
    );
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge::route_config::RouteConfigFile;
    use crate::transcode;

    const CFG: &str = r#"
upstreams:
  aggregator-core:
    endpoints: ["http://127.0.0.1:3100"]
routes:
  - id: health
    match: { path: /health, methods: [GET] }
    target: { kind: static, status: 200, body: '{"status":"ok"}' }
  - id: get-aggregated
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
"#;

    async fn build_edge() -> BffEdge {
        let cfg = RouteConfigFile::from_yaml(CFG).unwrap();
        let pool = transcode::load().unwrap();
        let router = Router::from_config(cfg.clone(), pool).unwrap();
        let upstreams = UpstreamRegistry::from_config(&cfg.upstreams).unwrap();
        BffEdge::new(EdgeState::new(router, upstreams))
    }

    async fn collect_text(resp: Response<BodyT>) -> (u16, String, Option<String>) {
        let status = resp.status().as_u16();
        let ct = resp
            .headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned());
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(body.to_vec()).unwrap();
        (status, text, ct)
    }

    // Note: `hyper::body::Incoming` is not user-constructible, so tests that
    // need to exercise `dispatch` end-to-end wire a real hyper server (PR 7
    // integration tests). These unit tests cover helpers directly.
    #[tokio::test]
    async fn router_not_found_returns_problem() {
        let edge = build_edge().await;
        // Invoke `dispatch` indirectly by running the router match and
        // problem_response helpers. This is a smoke test; full HTTP
        // integration runs in the binary.
        let state = edge.state.clone();
        let outcome = state
            .router
            .match_request(&Method::GET, "/does/not/exist");
        assert!(matches!(outcome, RouteMatch::NotFound));
        let pd = route_not_found("GET", "/does/not/exist");
        let resp = problem_response(pd);
        let (status, text, ct) = collect_text(resp).await;
        assert_eq!(status, 404);
        assert!(text.contains("urn:problem-type:not-found"));
        assert_eq!(ct.as_deref(), Some(PROBLEM_CONTENT_TYPE));
    }

    #[tokio::test]
    async fn static_route_returns_configured_body() {
        let resp = static_response(200, "{\"status\":\"ok\"}".into());
        let (status, text, ct) = collect_text(resp).await;
        assert_eq!(status, 200);
        assert_eq!(text, "{\"status\":\"ok\"}");
        assert_eq!(ct.as_deref(), Some("application/json"));
    }
}
