//! gRPC ↔ JSON transcoder.
//!
//! Plugs into Pingora's `ProxyHttp` hooks to convert REST/JSON requests to
//! framed gRPC wire bytes (request path) and gRPC responses back to JSON or
//! Server-Sent Events (response path). Driven by a `prost-reflect`
//! [`DescriptorPool`] loaded from a proto `FileDescriptorSet` compiled at
//! build time.

pub mod codec;
pub mod descriptors;
pub mod errors;
pub mod request;
pub mod response;
pub mod streaming;

pub use codec::{BytesCodec, BytesDecoder, BytesEncoder};

pub use descriptors::{decode_pool, install, load, pool_bytes};
pub use errors::{
    app_error_to_problem, grpc_status_to_app_error, method_not_allowed, route_not_found,
    upstream_unavailable, FieldViolation, GrpcIntoProblem, IntoProblem, ProblemDetail,
    PROBLEM_CONTENT_TYPE,
};
#[cfg(feature = "axum-response")]
pub use errors::fallback_handler;
pub use request::{
    encode as encode_request, frame, parse_query, transcode as transcode_request, Encoded,
    FramedRequest, TranscodeRequest,
};
pub use response::{
    trailer_to_status, transcode as transcode_response, transcode_unframed, unframe,
    TranscodeResponse, TranscodedResponse,
};
pub use streaming::{
    body_from_error as sse_body_from_error, decode_message_to_json, into_sse_stream, sse_body,
    sse_error_event, sse_event, sse_keepalive, SseStream, DEFAULT_KEEPALIVE, SSE_CONTENT_TYPE,
};

/// Generated proto types for the bundled fixture descriptor.
///
/// Used internally by the library's own tests (`response`, `streaming`).
/// Not part of the stable public API; consumers should generate and use
/// their own proto types.
#[cfg(test)]
#[allow(clippy::derive_partial_eq_without_eq, clippy::large_enum_variant)]
pub(crate) mod proto {
    tonic::include_proto!("fixture.v1");
}
