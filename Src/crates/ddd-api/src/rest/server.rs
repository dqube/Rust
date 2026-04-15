//! Configurable REST / HTTP server builder with graceful shutdown.

use axum::Router;
use ddd_shared_kernel::AppResult;
use std::net::SocketAddr;
use std::time::Duration;

/// Builder for an Axum HTTP server.
pub struct RestServer {
    addr: SocketAddr,
    router: Router,
    /// Maximum time to wait for in-flight requests to complete after a
    /// shutdown signal is received.
    shutdown_timeout: Duration,
}

impl RestServer {
    /// Create a new server builder listening on `0.0.0.0:8080`.
    pub fn new() -> Self {
        Self {
            addr: ([0, 0, 0, 0], 8080).into(),
            router: Router::new(),
            shutdown_timeout: Duration::from_secs(30),
        }
    }

    /// Override the listen port.
    pub fn with_port(mut self, port: u16) -> Self {
        self.addr.set_port(port);
        self
    }

    /// Override the full listen address.
    pub fn with_addr(mut self, addr: SocketAddr) -> Self {
        self.addr = addr;
        self
    }

    /// Replace the router entirely.
    pub fn with_router(mut self, router: Router) -> Self {
        self.router = router;
        self
    }

    /// Merge another router into the current one.
    pub fn merge(mut self, router: Router) -> Self {
        self.router = self.router.merge(router);
        self
    }

    /// Apply a tower layer to the router.
    pub fn layer<L>(mut self, layer: L) -> Self
    where
        L: tower::Layer<axum::routing::Route> + Clone + Send + Sync + 'static,
        L::Service: tower::Service<
                axum::http::Request<axum::body::Body>,
                Response = axum::response::Response,
            > + Clone
            + Send
            + Sync
            + 'static,
        <L::Service as tower::Service<axum::http::Request<axum::body::Body>>>::Future:
            Send + 'static,
        <L::Service as tower::Service<axum::http::Request<axum::body::Body>>>::Error:
            Into<std::convert::Infallible> + Send,
    {
        self.router = self.router.layer(layer);
        self
    }

    /// Set the maximum time to wait for in-flight requests to drain after a
    /// shutdown signal is received. Defaults to 30 seconds.
    pub fn with_shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    /// Start serving with graceful shutdown.
    ///
    /// The server listens for `SIGTERM` and `SIGINT` (Ctrl-C). On the first
    /// signal it stops accepting new connections and waits up to
    /// [`shutdown_timeout`](Self::with_shutdown_timeout) for in-flight
    /// requests to complete before returning.
    pub async fn run(self) -> AppResult<()> {
        tracing::info!(addr = %self.addr, "starting REST server");

        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .map_err(|e| ddd_shared_kernel::AppError::internal(format!("bind error: {e}")))?;

        let shutdown_timeout = self.shutdown_timeout;

        axum::serve(listener, self.router)
            .with_graceful_shutdown(shutdown_signal("REST"))
            .await
            .map_err(|e| ddd_shared_kernel::AppError::internal(format!("REST server error: {e}")))?;

        // Give in-flight connections time to drain.
        tracing::info!(
            timeout_secs = shutdown_timeout.as_secs(),
            "REST server draining connections"
        );
        tokio::time::sleep(shutdown_timeout).await;

        tracing::info!("REST server stopped");
        Ok(())
    }
}

impl Default for RestServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Waits for `SIGTERM` or `SIGINT`, whichever arrives first.
async fn shutdown_signal(label: &str) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => tracing::info!("{label} server received SIGINT"),
        () = terminate => tracing::info!("{label} server received SIGTERM"),
    }
}
