//! Structured JSON logging.

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Install a JSON `tracing_subscriber` with the given default level.
///
/// Respects `RUST_LOG` when set; falls back to `level` otherwise.
pub fn init_logging(level: &str) -> Result<(), Box<dyn std::error::Error>> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json())
        .try_init()?;
    Ok(())
}
