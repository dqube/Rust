//! Generic HTTP reverse proxy for BFF gateways.
//!
//! Forwards every request under a configured path prefix to a downstream HTTP
//! service, stripping the prefix before forwarding. New downstream endpoints
//! become available automatically without BFF code changes.
//!
//! Feature-gated on `axum-response` (requires `reqwest`).

use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};

use crate::transcode::upstream_unavailable;

/// Headers that must not be forwarded to the downstream service.
const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
    "host",
    "content-length",
];

/// State for the generic HTTP reverse proxy.
#[derive(Clone)]
pub struct ProxyState {
    pub client: reqwest::Client,
    /// Base URL of the downstream service (e.g. `http://localhost:8080`).
    pub upstream_base: Arc<String>,
    /// URL prefix to strip before forwarding (e.g. `/admin`).
    pub strip_prefix: Arc<String>,
}

impl ProxyState {
    /// Build a [`ProxyState`] with a fresh `reqwest::Client`.
    pub fn new(
        upstream_base: String,
        strip_prefix: String,
        timeout: std::time::Duration,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("failed to build proxy reqwest client");
        Self {
            client,
            upstream_base: Arc::new(upstream_base),
            strip_prefix: Arc::new(strip_prefix),
        }
    }
}

/// Catch-all axum proxy handler.
///
/// Mount at a wildcard route so every request under the prefix is forwarded:
///
/// ```ignore
/// use axum::routing::any;
/// use ddd_bff::proxy::{ProxyState, proxy_handler};
///
/// let proxy = ProxyState::new(
///     "http://upstream:8080".into(),
///     "/api".into(),
///     std::time::Duration::from_secs(5),
/// );
/// let app = axum::Router::new()
///     .route("/api/{*path}", any(proxy_handler))
///     .with_state(proxy);
/// ```
pub async fn proxy_handler(
    State(state): State<ProxyState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let path = uri.path();
    let downstream_path = path
        .strip_prefix(state.strip_prefix.as_str())
        .unwrap_or(path);
    let query = uri.query().map(|q| format!("?{q}")).unwrap_or_default();
    let target = format!("{}{}{}", state.upstream_base, downstream_path, query);

    tracing::debug!(%method, from = %path, to = %target, "proxy forward");

    let reqwest_method = match reqwest::Method::from_bytes(method.as_str().as_bytes()) {
        Ok(m) => m,
        Err(_) => return bad_gateway("invalid method"),
    };

    let mut req = state.client.request(reqwest_method, &target);
    for (name, value) in headers.iter() {
        if HOP_BY_HOP.contains(&name.as_str()) {
            continue;
        }
        if let Ok(v) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
            req = req.header(name.as_str(), v);
        }
    }
    if !body.is_empty() {
        req = req.body(body.to_vec());
    }

    let resp = match req.send().await {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!(error = %err, target = %target, "upstream unreachable");
            return bad_gateway(&format!("upstream unreachable: {err}"));
        }
    };

    let status =
        StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut out_headers = HeaderMap::new();
    for (name, value) in resp.headers().iter() {
        if HOP_BY_HOP.contains(&name.as_str()) {
            continue;
        }
        if let (Ok(n), Ok(v)) = (
            HeaderName::from_bytes(name.as_str().as_bytes()),
            HeaderValue::from_bytes(value.as_bytes()),
        ) {
            out_headers.insert(n, v);
        }
    }

    let bytes = resp.bytes().await.unwrap_or_default();
    (status, out_headers, Body::from(bytes)).into_response()
}

fn bad_gateway(detail: &str) -> Response {
    let pd = upstream_unavailable(detail);
    pd.into_response()
}
