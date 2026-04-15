//! gRPC server-streaming → Server-Sent Events.
//!
//! Converts a tonic [`Streaming`] of unframed proto payloads (produced by
//! [`tonic::client::Grpc::server_streaming`] when called with the
//! [`super::codec::BytesCodec`]) into an HTTP body of `text/event-stream`
//! frames suitable for browser `EventSource` clients.
//!
//! Wire format (per [W3C SSE](https://html.spec.whatwg.org/multipage/server-sent-events.html)):
//!
//! ```text
//! event: <method-name>\n
//! data: <json>\n
//! \n
//! ```
//!
//! On a non-zero `grpc-status` trailer (or any transport-level failure
//! reading the stream), the bridge emits a final
//! `event: error\ndata: <ProblemDetail>\n\n` frame and closes the
//! connection. Idle keep-alives are sent as SSE comments (`:\n\n`) every
//! `keepalive_interval`.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use ddd_shared_kernel::AppError;
use futures::Stream;
use hyper::body::Frame;
use prost_reflect::MethodDescriptor;
use tokio::time::{interval, Interval, MissedTickBehavior};
use tonic::Streaming;

use super::errors::{app_error_to_problem, grpc_status_to_app_error, ProblemDetail};
use super::response::{transcode_unframed, TranscodedResponse};

/// MIME type used by SSE responses.
pub const SSE_CONTENT_TYPE: &str = "text/event-stream";

/// Default keep-alive interval (matches the design doc).
pub const DEFAULT_KEEPALIVE: Duration = Duration::from_secs(15);

/// Format a single SSE event whose `data:` payload is `json` and whose
/// `event:` name is `event_name`.
pub fn sse_event(event_name: &str, json: &[u8]) -> Bytes {
    // SSE only supports a single line per `data:` field; embedded newlines
    // must be split into multiple `data:` lines per the spec. proto3 JSON
    // serialisation is a single line, but defend against pretty-printed
    // payloads anyway.
    let mut buf = Vec::with_capacity(json.len() + event_name.len() + 16);
    buf.extend_from_slice(b"event: ");
    buf.extend_from_slice(sanitise_event_name(event_name).as_bytes());
    buf.push(b'\n');
    for line in json.split(|b| *b == b'\n') {
        buf.extend_from_slice(b"data: ");
        buf.extend_from_slice(line);
        buf.push(b'\n');
    }
    buf.push(b'\n');
    Bytes::from(buf)
}

/// Format an SSE error event carrying a serialised [`ProblemDetail`].
pub fn sse_error_event(pd: &ProblemDetail) -> Bytes {
    sse_event("error", &pd.to_body())
}

/// Format an SSE keep-alive comment line.
pub fn sse_keepalive() -> Bytes {
    Bytes::from_static(b":\n\n")
}

/// Strip any whitespace / control characters from the event name so it
/// cannot break the SSE framing.
fn sanitise_event_name(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .collect()
}

/// Decode a single proto message (already unframed) to JSON using the given
/// method's output descriptor.
#[allow(clippy::result_large_err)]
pub fn decode_message_to_json(
    method_desc: &MethodDescriptor,
    proto_bytes: &[u8],
) -> Result<Vec<u8>, ProblemDetail> {
    match transcode_unframed(method_desc, proto_bytes, 200) {
        TranscodedResponse::Json { body, .. } => Ok(body),
        TranscodedResponse::Problem(pd) => Err(pd),
    }
}

/// Wrap a tonic [`Streaming`] of unframed proto bytes in an SSE-emitting
/// stream.
///
/// Each upstream proto message becomes one `event: <method>\ndata: <json>`
/// frame. A keep-alive comment is emitted whenever the upstream is idle for
/// `keepalive`. Errors (transport or non-zero `grpc-status` trailer)
/// produce a final `event: error\ndata: <ProblemDetail>\n\n` frame, after
/// which the stream ends.
pub fn into_sse_stream(
    upstream: Streaming<Bytes>,
    method_desc: MethodDescriptor,
    keepalive: Duration,
) -> SseStream {
    let mut tick = interval(keepalive);
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    SseStream {
        upstream,
        method_desc,
        keepalive: tick,
        finished: false,
    }
}

/// Adapter implementing [`futures::Stream`] of `Result<Frame<Bytes>, _>`
/// suitable for [`http_body_util::StreamBody`].
pub struct SseStream {
    upstream: Streaming<Bytes>,
    method_desc: MethodDescriptor,
    keepalive: Interval,
    finished: bool,
}

impl SseStream {
    /// Event name used for upstream messages (the gRPC method name).
    fn event_name(&self) -> String {
        self.method_desc.name().to_owned()
    }
}

impl Stream for SseStream {
    type Item = Result<Frame<Bytes>, io::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.finished {
            return Poll::Ready(None);
        }

        // Pull next upstream message first (preferred over keep-alive).
        match Pin::new(&mut this.upstream).poll_next(cx) {
            Poll::Ready(Some(Ok(proto_bytes))) => {
                let event_name = this.event_name();
                let bytes = match decode_message_to_json(&this.method_desc, &proto_bytes) {
                    Ok(json) => sse_event(&event_name, &json),
                    Err(pd) => {
                        this.finished = true;
                        sse_error_event(&pd)
                    }
                };
                Poll::Ready(Some(Ok(Frame::data(bytes))))
            }
            Poll::Ready(Some(Err(status))) => {
                this.finished = true;
                let app_err = grpc_status_to_app_error(status);
                let pd = app_error_to_problem(&app_err);
                Poll::Ready(Some(Ok(Frame::data(sse_error_event(&pd)))))
            }
            Poll::Ready(None) => {
                this.finished = true;
                Poll::Ready(None)
            }
            Poll::Pending => {
                // Idle — emit a keep-alive comment when the timer fires.
                match this.keepalive.poll_tick(cx) {
                    Poll::Ready(_) => Poll::Ready(Some(Ok(Frame::data(sse_keepalive())))),
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
}

/// Convenience: turn a transport-level failure (e.g. upstream connect) into
/// a one-shot SSE body that emits a single `error` event and ends.
pub fn one_shot_error(pd: ProblemDetail) -> Bytes {
    sse_error_event(&pd)
}

/// Helper used by tests and integrators that want to build the body
/// outside the edge service. Wraps the [`SseStream`] in a
/// [`http_body_util::StreamBody`] and returns it as an unsync-boxed body
/// matching [`crate::edge::service::BodyT`]. Unsync is required because
/// [`tonic::Streaming`] is not `Sync`.
pub fn sse_body(
    stream: SseStream,
) -> http_body_util::combinators::UnsyncBoxBody<Bytes, io::Error> {
    use http_body_util::{BodyExt, StreamBody};
    StreamBody::new(stream).boxed_unsync()
}

/// Wrap an [`AppError`] into a one-shot SSE body. Useful when the upstream
/// call fails before any frames have been received.
pub fn body_from_error(
    err: &AppError,
) -> http_body_util::combinators::UnsyncBoxBody<Bytes, io::Error> {
    use http_body_util::{BodyExt, Full};
    let pd = app_error_to_problem(err);
    Full::new(one_shot_error(pd))
        .map_err(|never| match never {})
        .boxed_unsync()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::proto;
    use crate::transcode;
    use prost::Message;

    fn method_desc(service: &str, method: &str) -> MethodDescriptor {
        let pool = transcode::load().unwrap();
        pool.get_service_by_name(service)
            .and_then(|s| s.methods().find(|m| m.name() == method))
            .unwrap()
    }

    #[test]
    fn sse_event_formats_single_line_json() {
        let bytes = sse_event("OrderEvent", br#"{"id":"o-1"}"#);
        let s = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(s, "event: OrderEvent\ndata: {\"id\":\"o-1\"}\n\n");
    }

    #[test]
    fn sse_event_splits_embedded_newlines() {
        let bytes = sse_event("Tick", b"line1\nline2");
        let s = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(s, "event: Tick\ndata: line1\ndata: line2\n\n");
    }

    #[test]
    fn sanitise_event_name_strips_newlines() {
        let bytes = sse_event("evil\nname", b"{}");
        let s = std::str::from_utf8(&bytes).unwrap();
        assert!(s.starts_with("event: evilname\n"));
    }

    #[test]
    fn sse_error_event_carries_problem_json() {
        let pd = ProblemDetail::new(404, "Not Found", "missing");
        let bytes = sse_error_event(&pd);
        let s = std::str::from_utf8(&bytes).unwrap();
        assert!(s.starts_with("event: error\n"));
        assert!(s.contains("\"status\":404"));
        assert!(s.ends_with("\n\n"));
    }

    #[test]
    fn keepalive_is_sse_comment() {
        assert_eq!(&sse_keepalive()[..], b":\n\n");
    }

    #[test]
    fn decode_message_to_json_uses_camel_case() {
        // EchoResponse has camelCase-mapped fields via proto3 JSON.
        let resp = proto::EchoResponse {
            message: "hi".into(),
            code: 9,
        };
        let mut buf = Vec::new();
        resp.encode(&mut buf).unwrap();

        let md = method_desc("fixture.v1.FixtureService", "Echo");
        let json = decode_message_to_json(&md, &buf).expect("decoded");
        let text = std::str::from_utf8(&json).unwrap();
        assert!(text.contains("\"message\":\"hi\""));
        assert!(text.contains("\"code\":9"));
    }
}
