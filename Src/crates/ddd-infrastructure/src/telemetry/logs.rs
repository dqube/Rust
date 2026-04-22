//! OTLP log-export pipeline.
//!
//! Builds an [`SdkLoggerProvider`] that ships every `tracing` event (after
//! it passes the global `EnvFilter`) to the same OTLP collector that
//! receives spans, so logs land alongside traces in Loki / Tempo.
//!
//! Exposed via the [`OpenTelemetryTracingBridge`] layer that
//! [`super::tracing::init_tracing`] attaches when
//! `OTEL_LOGS_EXPORTER=otlp` (the default).
//!
//! Set `OTEL_LOGS_EXPORTER=none` to disable the bridge — useful for
//! local development without an OTel collector.

use std::sync::OnceLock;

use ddd_shared_kernel::{AppError, AppResult};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{logs::SdkLoggerProvider, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;

static LOGGER_PROVIDER: OnceLock<SdkLoggerProvider> = OnceLock::new();

/// Returns `true` when OTLP log export is enabled for this process.
///
/// Reads `OTEL_LOGS_EXPORTER`; treats `otlp` (default) as on and `none`
/// as off.  Any other value is treated as on so misconfiguration fails
/// loudly at the collector rather than silently dropping logs.
pub fn otlp_logs_enabled() -> bool {
    !matches!(
        std::env::var("OTEL_LOGS_EXPORTER")
            .unwrap_or_else(|_| "otlp".to_owned())
            .as_str(),
        "none" | "off" | ""
    )
}

/// Build and install the OTLP log pipeline.  Idempotent — repeated calls
/// return a clone of the already-installed provider.
///
/// # Errors
/// Returns [`AppError::internal`] when the OTLP gRPC exporter cannot be
/// constructed.
pub fn init_log_pipeline(service_name: &str) -> AppResult<SdkLoggerProvider> {
    if let Some(existing) = LOGGER_PROVIDER.get() {
        return Ok(existing.clone());
    }

    let exporter = opentelemetry_otlp::LogExporter::builder()
        .with_tonic()
        .with_endpoint(
            std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:4317".to_owned()),
        )
        .build()
        .map_err(|e| AppError::internal(format!("otlp log exporter: {e}")))?;

    let resource = Resource::builder_empty()
        .with_attribute(opentelemetry::KeyValue::new(
            SERVICE_NAME,
            service_name.to_owned(),
        ))
        .build();

    let provider = SdkLoggerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build();

    let _ = LOGGER_PROVIDER.set(provider.clone());
    Ok(provider)
}

/// Flush and shut down the OTLP log provider, if one was installed.
pub fn shutdown_logs() {
    if let Some(provider) = LOGGER_PROVIDER.get() {
        let _ = provider.shutdown();
    }
}
