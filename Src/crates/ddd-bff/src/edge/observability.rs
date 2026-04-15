//! Per-request observability for the BFF edge.
//!
//! Provides:
//! - `x-request-id` extraction (or generation via UUID v7).
//! - In-flight gauge bookkeeping.
//! - One structured log line per request from `finish()`.
//! - Prometheus counters/histograms for HTTP and gRPC upstream calls.
//!
//! The edge service constructs a [`RequestObs`] at the top of `dispatch`,
//! mutates it as the request progresses (route id, upstream timing), and
//! calls [`RequestObs::finish`] before returning the response.

use std::time::Instant;

use http::{HeaderMap, HeaderValue, Method, Response};
use uuid::Uuid;

use crate::edge::service::BodyT;
use crate::metrics::BFF_METRICS;

/// Header carrying the correlation id propagated to downstream services.
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Route id label used when no route matched.
pub const UNKNOWN_ROUTE: &str = "unknown";

/// Per-request observability state.
pub struct RequestObs {
    /// Correlation id (read from `x-request-id` or generated as UUID v7).
    pub request_id: String,
    /// Request method (used for metric labels and the access log line).
    pub method: Method,
    /// Request path (logged but not used as a metric label — too high
    /// cardinality).
    pub path: String,
    start: Instant,
}

impl RequestObs {
    /// Build a [`RequestObs`] from the incoming request, generating a
    /// request id if the client did not supply one.
    pub fn start(method: &Method, path: &str, headers: &HeaderMap) -> Self {
        let request_id = headers
            .get(REQUEST_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned())
            .unwrap_or_else(|| Uuid::now_v7().to_string());

        BFF_METRICS.in_flight.inc();

        Self {
            request_id,
            method: method.clone(),
            path: path.to_owned(),
            start: Instant::now(),
        }
    }

    /// Ensure the supplied header map contains an `x-request-id`. Used to
    /// propagate the correlation id to downstream gRPC calls.
    pub fn ensure_in_headers(&self, headers: &mut HeaderMap) {
        if !headers.contains_key(REQUEST_ID_HEADER) {
            if let Ok(v) = HeaderValue::from_str(&self.request_id) {
                headers.insert(REQUEST_ID_HEADER, v);
            }
        }
    }

    /// Stamp the response with `x-request-id` so the client can correlate.
    pub fn stamp_response(&self, resp: &mut Response<BodyT>) {
        if let Ok(v) = HeaderValue::from_str(&self.request_id) {
            resp.headers_mut().insert(REQUEST_ID_HEADER, v);
        }
    }

    /// Record per-request metrics and emit one structured access log line.
    pub fn finish(self, route_id: &str, status: u16) {
        let elapsed = self.start.elapsed();
        BFF_METRICS.in_flight.dec();
        BFF_METRICS
            .request_count
            .with_label_values(&[route_id, self.method.as_str(), &status.to_string()])
            .inc();
        BFF_METRICS
            .request_duration
            .with_label_values(&[route_id, self.method.as_str()])
            .observe(elapsed.as_secs_f64());

        tracing::info!(
            request_id = %self.request_id,
            method = %self.method,
            path = %self.path,
            route = route_id,
            status = status,
            duration_ms = %format_args!("{:.3}", elapsed.as_secs_f64() * 1000.0),
            "edge request"
        );
    }
}

/// Record metrics for a single upstream gRPC call.
pub fn record_upstream(upstream: &str, grpc_status: i32, elapsed_secs: f64) {
    BFF_METRICS
        .upstream_count
        .with_label_values(&[upstream, &grpc_status.to_string()])
        .inc();
    BFF_METRICS
        .upstream_duration
        .with_label_values(&[upstream])
        .observe(elapsed_secs);
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderName;

    #[test]
    fn extracts_existing_request_id() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static(REQUEST_ID_HEADER),
            HeaderValue::from_static("abc-123"),
        );
        let obs = RequestObs::start(&Method::GET, "/x", &headers);
        assert_eq!(obs.request_id, "abc-123");
        obs.finish("test", 200);
    }

    #[test]
    fn generates_request_id_when_absent() {
        let obs = RequestObs::start(&Method::POST, "/y", &HeaderMap::new());
        assert!(!obs.request_id.is_empty());
        // UUID v7 string length is 36.
        assert_eq!(obs.request_id.len(), 36);
        obs.finish("test", 201);
    }

    #[test]
    fn ensure_in_headers_sets_when_missing() {
        let obs = RequestObs::start(&Method::GET, "/z", &HeaderMap::new());
        let mut downstream = HeaderMap::new();
        obs.ensure_in_headers(&mut downstream);
        assert_eq!(
            downstream.get(REQUEST_ID_HEADER).unwrap().to_str().unwrap(),
            obs.request_id
        );
        obs.finish("test", 200);
    }
}
