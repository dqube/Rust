//! Prometheus metrics for the BFF.
//!
//! Registered counters/histograms/gauges are consumed by the edge
//! observability hooks (PR 9) and exposed on a dedicated loopback
//! `/metrics` scrape endpoint.

use lazy_static::lazy_static;
use prometheus::{
    self, Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGauge, Opts, Registry,
    TextEncoder,
};

lazy_static! {
    /// Singleton metrics instance.
    pub static ref BFF_METRICS: BffMetrics = BffMetrics::new();
}

/// BFF-specific Prometheus metrics.
pub struct BffMetrics {
    /// Total number of HTTP requests, labelled by `route`, `method`, `status`.
    pub request_count: IntCounterVec,
    /// Request duration in seconds, labelled by `route`, `method`.
    pub request_duration: HistogramVec,
    /// Number of requests currently being processed.
    pub in_flight: IntGauge,
    /// Total number of gRPC upstream requests, labelled by `upstream`, `grpc_status`.
    pub upstream_count: IntCounterVec,
    /// gRPC upstream request duration in seconds, labelled by `upstream`.
    pub upstream_duration: HistogramVec,
    registry: Registry,
}

impl BffMetrics {
    fn new() -> Self {
        let registry = Registry::new();

        let request_count = IntCounterVec::new(
            Opts::new("bff_http_requests_total", "Total HTTP requests"),
            &["route", "method", "status"],
        )
        .expect("metric creation");

        let request_duration = HistogramVec::new(
            HistogramOpts::new(
                "bff_http_request_duration_seconds",
                "HTTP request duration in seconds",
            )
            .buckets(vec![
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            &["route", "method"],
        )
        .expect("metric creation");

        let in_flight = IntGauge::new(
            "bff_http_requests_in_flight",
            "Number of HTTP requests currently being processed",
        )
        .expect("metric creation");

        let upstream_count = IntCounterVec::new(
            Opts::new(
                "bff_grpc_upstream_requests_total",
                "Total gRPC upstream requests",
            ),
            &["upstream", "grpc_status"],
        )
        .expect("metric creation");

        let upstream_duration = HistogramVec::new(
            HistogramOpts::new(
                "bff_grpc_upstream_duration_seconds",
                "gRPC upstream request duration in seconds",
            )
            .buckets(vec![
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            &["upstream"],
        )
        .expect("metric creation");

        registry
            .register(Box::new(request_count.clone()))
            .expect("metric registration");
        registry
            .register(Box::new(request_duration.clone()))
            .expect("metric registration");
        registry
            .register(Box::new(in_flight.clone()))
            .expect("metric registration");
        registry
            .register(Box::new(upstream_count.clone()))
            .expect("metric registration");
        registry
            .register(Box::new(upstream_duration.clone()))
            .expect("metric registration");

        Self {
            request_count,
            request_duration,
            in_flight,
            upstream_count,
            upstream_duration,
            registry,
        }
    }

    /// Encode all metrics in the Prometheus text format.
    pub fn encode(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buf = Vec::new();
        encoder.encode(&metric_families, &mut buf).unwrap_or(());
        String::from_utf8(buf).unwrap_or_default()
    }
}

/// MIME type for Prometheus text-format responses.
pub const PROMETHEUS_CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

/// Axum handler that serves all registered BFF metrics in Prometheus text
/// format. Mount at `/metrics`.
///
/// ```ignore
/// use axum::routing::get;
/// use ddd_bff::metrics::metrics_handler;
///
/// let app = axum::Router::new().route("/metrics", get(metrics_handler));
/// ```
#[cfg(feature = "axum-response")]
pub async fn metrics_handler() -> impl axum::response::IntoResponse {
    (
        axum::http::StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            PROMETHEUS_CONTENT_TYPE,
        )],
        BFF_METRICS.encode(),
    )
}
