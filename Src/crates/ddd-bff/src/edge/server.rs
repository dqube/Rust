//! Hyper accept loop for the BFF edge.
//!
//! Binds a [`TcpListener`] and serves [`BffEdge`] connections via
//! `hyper-util`'s auto HTTP/1+HTTP/2 builder. Stops accepting on
//! cancellation and gracefully drains in-flight connections.

use std::net::SocketAddr;

use hyper::service::Service;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use super::service::BffEdge;

/// Bind and serve the edge until `token` is cancelled.
pub async fn serve(
    addr: SocketAddr,
    edge: BffEdge,
    token: CancellationToken,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "edge listening");

    let builder = auto::Builder::new(TokioExecutor::new());

    loop {
        tokio::select! {
            _ = token.cancelled() => {
                tracing::info!("edge: cancellation received, no longer accepting connections");
                break;
            }
            accepted = listener.accept() => {
                let (stream, peer) = match accepted {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!(error = %e, "edge accept error");
                        continue;
                    }
                };
                let io = TokioIo::new(stream);
                let svc = ServiceCloneWrapper(edge.clone());
                let builder = builder.clone();
                let conn_token = token.clone();
                tokio::spawn(async move {
                    let conn = builder.serve_connection(io, svc);
                    tokio::pin!(conn);
                    tokio::select! {
                        res = conn.as_mut() => {
                            if let Err(e) = res {
                                tracing::debug!(peer = %peer, error = %e, "edge connection ended");
                            }
                        }
                        _ = conn_token.cancelled() => {
                            conn.as_mut().graceful_shutdown();
                            let _ = conn.await;
                        }
                    }
                });
            }
        }
    }
    Ok(())
}

/// Adapter so `BffEdge` (which implements `Service<Request, ...>`) can be
/// used directly with `hyper-util`'s auto builder. Cloning the edge clones
/// its `Arc`s — cheap.
#[derive(Clone)]
struct ServiceCloneWrapper(BffEdge);

impl<R> Service<R> for ServiceCloneWrapper
where
    BffEdge: Service<R>,
{
    type Response = <BffEdge as Service<R>>::Response;
    type Error = <BffEdge as Service<R>>::Error;
    type Future = <BffEdge as Service<R>>::Future;

    fn call(&self, req: R) -> Self::Future {
        self.0.call(req)
    }
}
