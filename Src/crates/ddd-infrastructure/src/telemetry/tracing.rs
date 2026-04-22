//! OpenTelemetry tracing setup.
//!
//! Initialises the OTLP span exporter, the W3C trace-context propagator,
//! the `tracing` ↔ OpenTelemetry bridge, and (when
//! `OTEL_LOGS_EXPORTER=otlp`, the default) the OTLP log-export pipeline
//! from [`super::logs`].  Also installs a panic hook that flushes
//! telemetry before the process aborts so the last span — usually the
//! cause of the crash — is not lost.

use std::sync::OnceLock;

use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use super::logs::{init_log_pipeline, otlp_logs_enabled, shutdown_logs};

static TRACER_PROVIDER: OnceLock<SdkTracerProvider> = OnceLock::new();

/// Initialise the OTLP tracer, install a W3C trace-context propagator, and
/// wire `tracing` to OpenTelemetry via [`tracing_opentelemetry`].
///
/// Respects `RUST_LOG`; defaults to `info`.
pub fn init_tracing(service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(
            std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:4317".to_owned()),
        )
        .build()?;

    let resource = Resource::builder_empty()
        .with_attribute(opentelemetry::KeyValue::new(
            SERVICE_NAME,
            service_name.to_owned(),
        ))
        .build();

    let provider = SdkTracerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build();

    global::set_tracer_provider(provider.clone());
    let tracer = provider.tracer(service_name.to_string());
    let _ = TRACER_PROVIDER.set(provider);

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Optional OTLP log bridge — wired only when enabled and the exporter
    // builds successfully.  A failure to build the log pipeline must not
    // prevent traces from initialising; surface it as a stderr warning so
    // the operator notices without taking down the service.
    let log_bridge = if otlp_logs_enabled() {
        match init_log_pipeline(service_name) {
            Ok(provider) => Some(OpenTelemetryTracingBridge::new(&provider)),
            Err(e) => {
                eprintln!("warn: OTLP log pipeline disabled: {e}");
                None
            }
        }
    } else {
        None
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json())
        .with(otel_layer)
        .with(log_bridge)
        .try_init()?;

    install_panic_hook();

    Ok(())
}

/// Flush and shut down the global tracer provider.
pub fn shutdown_tracing() {
    if let Some(provider) = TRACER_PROVIDER.get() {
        let _ = provider.shutdown();
    }
    shutdown_logs();
}

/// Wrap the existing panic hook with one that flushes telemetry before
/// delegating.  Idempotent — installs at most once per process.
fn install_panic_hook() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    if INSTALLED.set(()).is_err() {
        return;
    }

    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Best-effort flush — log the panic before tearing the providers
        // down so the final event makes it into the OTLP pipeline.
        tracing::error!(panic = %info, "panic, flushing telemetry");
        shutdown_tracing();
        original(info);
    }));
}

