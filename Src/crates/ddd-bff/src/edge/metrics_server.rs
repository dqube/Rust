//! Tiny hyper service exposing Prometheus metrics on `/metrics`.
//!
//! Bound to loopback by default (the BFF process). Designed to be served
//! alongside the edge with a shared cancellation token.

use std::convert::Infallible;
use std::net::SocketAddr;

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::metrics::{BFF_METRICS, PROMETHEUS_CONTENT_TYPE};

/// Body type used by the metrics scrape responses.
pub type ScrapeBody = BoxBody<Bytes, Infallible>;

async fn handle(req: Request<Incoming>) -> Result<Response<ScrapeBody>, Infallible> {
    let path = req.uri().path();
    let resp = match (req.method(), path) {
        (&http::Method::GET, "/metrics") => {
            let body = BFF_METRICS.encode();
            let mut r = Response::new(full_body(Bytes::from(body)));
            *r.status_mut() = StatusCode::OK;
            r.headers_mut().insert(
                http::header::CONTENT_TYPE,
                http::HeaderValue::from_static(PROMETHEUS_CONTENT_TYPE),
            );
            r
        }
        (&http::Method::GET, "/health") => {
            let mut r = Response::new(full_body(Bytes::from_static(b"{\"status\":\"ok\"}")));
            *r.status_mut() = StatusCode::OK;
            r.headers_mut().insert(
                http::header::CONTENT_TYPE,
                http::HeaderValue::from_static("application/json"),
            );
            r
        }
        _ => {
            let mut r = Response::new(full_body(Bytes::from_static(b"not found")));
            *r.status_mut() = StatusCode::NOT_FOUND;
            r
        }
    };
    Ok(resp)
}

fn full_body(bytes: Bytes) -> ScrapeBody {
    Full::new(bytes).boxed()
}

/// Bind and serve `/metrics` until `token` is cancelled.
pub async fn serve(addr: SocketAddr, token: CancellationToken) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "metrics scrape listening");

    let builder = auto::Builder::new(TokioExecutor::new());

    loop {
        tokio::select! {
            _ = token.cancelled() => {
                tracing::info!("metrics server: cancellation received, no longer accepting");
                break;
            }
            accepted = listener.accept() => {
                let (stream, _peer) = match accepted {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!(error = %e, "metrics accept error");
                        continue;
                    }
                };
                let io = TokioIo::new(stream);
                let svc = service_fn(handle);
                let builder = builder.clone();
                let conn_token = token.clone();
                tokio::spawn(async move {
                    let conn = builder.serve_connection(io, svc);
                    tokio::pin!(conn);
                    tokio::select! {
                        res = conn.as_mut() => {
                            if let Err(e) = res {
                                tracing::debug!(error = %e, "metrics connection ended");
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
