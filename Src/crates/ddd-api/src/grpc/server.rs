//! Configurable gRPC server builder with graceful shutdown.

use ddd_shared_kernel::AppResult;
use std::net::SocketAddr;
use std::time::Duration;

/// A builder for a tonic gRPC server.
pub struct GrpcServer {
    addr: SocketAddr,
    router: Option<tonic::transport::server::Router>,
    /// Maximum time to wait for in-flight RPCs to complete after a shutdown
    /// signal is received.
    shutdown_timeout: Duration,
}

impl GrpcServer {
    /// Create a new server builder listening on `0.0.0.0:50051`.
    pub fn new() -> Self {
        Self {
            addr: ([0, 0, 0, 0], 50051).into(),
            router: None,
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

    /// Set the tonic [`Router`] (built by the caller via
    /// `Server::builder().add_service(...)`).
    pub fn with_router(mut self, router: tonic::transport::server::Router) -> Self {
        self.router = Some(router);
        self
    }

    /// Set the maximum time to wait for in-flight RPCs to drain after a
    /// shutdown signal is received. Defaults to 30 seconds.
    pub fn with_shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    /// Start serving with graceful shutdown.
    ///
    /// The server listens for `SIGTERM` and `SIGINT` (Ctrl-C). On the first
    /// signal it stops accepting new connections and waits up to
    /// [`shutdown_timeout`](Self::with_shutdown_timeout) for in-flight RPCs
    /// to complete before returning.
    pub async fn run(self) -> AppResult<()> {
        let router = self
            .router
            .ok_or_else(|| ddd_shared_kernel::AppError::internal("no gRPC services registered"))?;

        tracing::info!(addr = %self.addr, "starting gRPC server");

        let shutdown_timeout = self.shutdown_timeout;

        router
            .serve_with_shutdown(self.addr, shutdown_signal("gRPC"))
            .await
            .map_err(|e| ddd_shared_kernel::AppError::internal(format!("gRPC server error: {e}")))?;

        // Give in-flight RPCs time to drain.
        tracing::info!(
            timeout_secs = shutdown_timeout.as_secs(),
            "gRPC server draining connections"
        );
        tokio::time::sleep(shutdown_timeout).await;

        tracing::info!("gRPC server stopped");
        Ok(())
    }
}

impl Default for GrpcServer {
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
