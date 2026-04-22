//! Telemetry: structured logging, OpenTelemetry tracing, and Prometheus
//! metrics.

pub mod logging;
pub mod logs;
pub mod metrics;
pub mod tracing;

pub use logging::init_logging;
pub use logs::{init_log_pipeline, shutdown_logs};
pub use metrics::{init_metrics, metrics_handler, Metrics};
pub use tracing::{init_tracing, shutdown_tracing};

/// One-shot initialiser for the full telemetry stack (logging + tracing +
/// metrics).
pub fn init_telemetry(service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    init_tracing(service_name)?;
    init_metrics()?;
    Ok(())
}

/// Flush and shut down telemetry providers.
pub fn shutdown_telemetry() {
    shutdown_tracing();
}
