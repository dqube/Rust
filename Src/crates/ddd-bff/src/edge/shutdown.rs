//! Graceful shutdown helpers for the BFF.
//!
//! Wires SIGTERM / SIGINT (Ctrl+C) to a shared
//! [`tokio_util::sync::CancellationToken`] consumed by the edge, the
//! aggregator, and the metrics scrape server. All three drain in parallel
//! when the token is cancelled.

use std::time::Duration;

use tokio_util::sync::CancellationToken;

/// Spawn a task that cancels `token` on SIGTERM or SIGINT.
pub fn install_signal_handler(token: CancellationToken) {
    tokio::spawn(async move {
        wait_for_shutdown_signal().await;
        token.cancel();
    });
}

/// Wait for SIGTERM / SIGINT.
pub async fn wait_for_shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "failed to install Ctrl+C handler");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                tracing::error!(error = %e, "failed to install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c   => tracing::info!("received SIGINT, shutting down"),
        _ = terminate => tracing::info!("received SIGTERM, shutting down"),
    }
}

/// Wait at most `timeout` for `f` to complete; return `true` if it did.
pub async fn drain_with_timeout<F: std::future::Future<Output = ()>>(
    label: &str,
    f: F,
    timeout: Duration,
) -> bool {
    match tokio::time::timeout(timeout, f).await {
        Ok(()) => {
            tracing::info!(component = label, "drain complete");
            true
        }
        Err(_) => {
            tracing::warn!(component = label, ?timeout, "drain timed out");
            false
        }
    }
}
