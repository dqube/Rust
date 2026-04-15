//! Tonic interceptor that propagates trace-context headers into outgoing
//! gRPC calls.
//!
//! Admin-bff (and any axum-based BFF) stores a [`RequestTraceContext`] as a
//! request extension via the observability middleware.  Because tonic clients
//! are created per-call or per-channel and have no access to axum extensions,
//! we use a [`tokio::task_local!`] to shuttle the trace context across the
//! async boundary.
//!
//! ## Usage
//!
//! ```ignore
//! use ddd_bff::middleware::tracing_interceptor::{TracingInterceptor, TRACE_CTX};
//! use ddd_bff::middleware::axum_observability::RequestTraceContext;
//!
//! // In your axum handler:
//! async fn my_handler(Extension(ctx): Extension<RequestTraceContext>) {
//!     let resp = TRACE_CTX
//!         .scope(ctx, async {
//!             client.some_rpc(request).await
//!         })
//!         .await;
//! }
//!
//! // Or apply the interceptor at channel level:
//! let channel = Channel::from_static("http://localhost:50051").connect().await?;
//! let client = MyServiceClient::with_interceptor(channel, TracingInterceptor);
//! ```
//!
//! Feature-gated on `axum-response` (same gate as the observability middleware
//! that produces the [`RequestTraceContext`]).

use tonic::{Request, Status};

use super::axum_observability::RequestTraceContext;

tokio::task_local! {
    /// Task-local carrying the current request's trace context.
    ///
    /// Set by the axum handler (or an outer middleware) before calling tonic
    /// clients.  Read by [`TracingInterceptor`].
    pub static TRACE_CTX: RequestTraceContext;
}

/// Tonic interceptor that injects `x-request-id`, `traceparent`, and
/// `tracestate` from the task-local [`TRACE_CTX`] into outgoing gRPC
/// metadata.
#[derive(Debug, Clone, Copy)]
pub struct TracingInterceptor;

impl tonic::service::Interceptor for TracingInterceptor {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        // Try to read from the task-local.  If not set (e.g. background
        // tasks, tests), silently skip — never fail the RPC.
        let ctx = TRACE_CTX.try_with(|c| c.clone()).ok();

        if let Some(ctx) = ctx {
            insert_meta(req.metadata_mut(), "x-request-id", &ctx.request_id);
            if let Some(ref tp) = ctx.traceparent {
                insert_meta(req.metadata_mut(), "traceparent", tp);
            }
            if let Some(ref ts) = ctx.tracestate {
                insert_meta(req.metadata_mut(), "tracestate", ts);
            }
        }

        Ok(req)
    }
}

fn insert_meta(meta: &mut tonic::metadata::MetadataMap, key: &str, value: &str) {
    if let (Ok(k), Ok(v)) = (
        tonic::metadata::MetadataKey::from_bytes(key.as_bytes()),
        value.parse(),
    ) {
        meta.insert(k, v);
    }
}
