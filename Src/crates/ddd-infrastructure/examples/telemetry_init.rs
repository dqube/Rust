use ddd_infrastructure::telemetry::{init_telemetry, shutdown_telemetry};
use tracing::{info, span, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize all telemetry concerns (Logging, OTLP Tracing, Prometheus Metrics)
    // By default, this reads from OTLP_ENDPOINT env var for tracing.
    println!("Initializing telemetry...");
    init_telemetry("example-service")?;

    // 2. Use structured logging
    info!(service = "example", version = "1.0", "Service started");

    // 3. Use tracing spans
    let root = span!(Level::INFO, "request_processor", request_id = "abc-123");
    let _enter = root.enter();

    info!("Processing nested operation...");
    do_work().await;

    // 4. Proper shutdown to flush traces
    println!("Shutting down telemetry...");
    shutdown_telemetry();

    Ok(())
}

async fn do_work() {
    let work_span = span!(Level::DEBUG, "expensive_operation");
    let _enter = work_span.enter();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    info!("Work completed");
}
