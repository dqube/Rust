//! Prometheus metrics registry and exposition handler.

use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_histogram_vec, CounterVec, Encoder, HistogramVec, TextEncoder,
};

lazy_static! {
    /// Total number of HTTP requests, labelled by `method`, `path`, `status`.
    pub static ref HTTP_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        "http_requests_total",
        "Total number of HTTP requests",
        &["method", "path", "status"]
    )
    .expect("register http_requests_total");

    /// HTTP request duration seconds, labelled by `method`, `path`.
    pub static ref HTTP_REQUEST_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "http_request_duration_seconds",
        "HTTP request duration in seconds",
        &["method", "path"]
    )
    .expect("register http_request_duration_seconds");

    /// Total number of gRPC requests, labelled by `service`, `method`, `status`.
    pub static ref GRPC_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        "grpc_requests_total",
        "Total number of gRPC requests",
        &["service", "method", "status"]
    )
    .expect("register grpc_requests_total");

    /// gRPC request duration seconds, labelled by `service`, `method`.
    pub static ref GRPC_REQUEST_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "grpc_request_duration_seconds",
        "gRPC request duration in seconds",
        &["service", "method"]
    )
    .expect("register grpc_request_duration_seconds");
}

/// Eagerly register all default metrics.
pub fn init_metrics() -> Result<(), prometheus::Error> {
    lazy_static::initialize(&HTTP_REQUESTS_TOTAL);
    lazy_static::initialize(&HTTP_REQUEST_DURATION_SECONDS);
    lazy_static::initialize(&GRPC_REQUESTS_TOTAL);
    lazy_static::initialize(&GRPC_REQUEST_DURATION_SECONDS);
    Ok(())
}

/// Return the current metrics in Prometheus exposition format.
pub fn metrics_handler() -> String {
    let encoder = TextEncoder::new();
    let mf = prometheus::gather();
    let mut buf = Vec::new();
    if encoder.encode(&mf, &mut buf).is_err() {
        return String::new();
    }
    String::from_utf8(buf).unwrap_or_default()
}

/// Thin wrapper exposing typed recording methods.
#[derive(Debug, Default, Clone, Copy)]
pub struct Metrics;

impl Metrics {
    /// Record one completed HTTP request.
    pub fn record_http(method: &str, path: &str, status: u16, duration_secs: f64) {
        HTTP_REQUESTS_TOTAL
            .with_label_values(&[method, path, &status.to_string()])
            .inc();
        HTTP_REQUEST_DURATION_SECONDS
            .with_label_values(&[method, path])
            .observe(duration_secs);
    }

    /// Record one completed gRPC request.
    pub fn record_grpc(service: &str, method: &str, status: &str, duration_secs: f64) {
        GRPC_REQUESTS_TOTAL
            .with_label_values(&[service, method, status])
            .inc();
        GRPC_REQUEST_DURATION_SECONDS
            .with_label_values(&[service, method])
            .observe(duration_secs);
    }
}
