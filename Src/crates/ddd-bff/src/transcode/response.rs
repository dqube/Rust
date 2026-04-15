//! gRPC response → REST JSON response.
//!
//! For unary routes, the upstream returns an HTTP/2 response with
//! `content-type: application/grpc`, a 5-byte-framed proto body, and a
//! trailer containing `grpc-status` (plus optionally `grpc-message`).
//!
//! This module:
//!
//! 1. Inspects `grpc-status`. If non-zero, maps to an [`AppError`] and
//!    then to an RFC 9457 [`ProblemDetail`].
//! 2. Strips the 5-byte frame.
//! 3. Decodes the proto bytes into a [`DynamicMessage`] using the method
//!    output descriptor.
//! 4. Serialises to JSON with the proto3 JSON mapping (camelCase) and
//!    emits explicit `null` for absent optional fields.
//!
//! The caller (the edge hook) is responsible for setting the HTTP status
//! and content-type based on the [`TranscodedResponse`] kind.

use bytes::Bytes;
use ddd_shared_kernel::AppError;
use prost_reflect::{
    Cardinality, DynamicMessage, Kind, MethodDescriptor, ReflectMessage, SerializeOptions,
};

use super::errors::{app_error_to_problem, grpc_status_to_app_error, ProblemDetail};

/// Terminal outcome of a gRPC response — either a JSON success body or a
/// Problem Details error body.
#[derive(Debug)]
pub enum TranscodedResponse {
    /// Success: JSON bytes + recommended HTTP status.
    Json {
        /// Serialised JSON payload.
        body: Vec<u8>,
        /// Recommended HTTP status code (e.g. 200, 201).
        status: u16,
    },
    /// Error: RFC 9457 Problem Details.
    Problem(ProblemDetail),
}

/// Parse a gRPC `grpc-status` trailer value. 0 is success.
fn parse_status(trailer: &str) -> i32 {
    trailer.trim().parse().unwrap_or(2) // 2 = Unknown
}

/// Convert a gRPC trailer pair into a [`tonic::Status`].
pub fn trailer_to_status(grpc_status: &str, grpc_message: Option<&str>) -> tonic::Status {
    let code = tonic::Code::from_i32(parse_status(grpc_status));
    tonic::Status::new(code, grpc_message.unwrap_or(""))
}

/// Inputs to the response transcoder for a unary call.
pub struct TranscodeResponse<'a> {
    /// Resolved gRPC method descriptor (used to decode the output).
    pub method_desc: &'a MethodDescriptor,
    /// Framed gRPC response body (5-byte frame + proto bytes).
    pub body: &'a [u8],
    /// Raw `grpc-status` trailer value. Missing trailer maps to `Unknown`.
    pub grpc_status: Option<&'a str>,
    /// Optional `grpc-message` trailer value.
    pub grpc_message: Option<&'a str>,
    /// Recommended HTTP status on success (e.g. 200 for GET, 201 for POST).
    pub success_status: u16,
}

/// Run the unary response transcode.
pub fn transcode(resp: TranscodeResponse<'_>) -> TranscodedResponse {
    // 1. Error trailer?
    if let Some(raw) = resp.grpc_status {
        let code = parse_status(raw);
        if code != 0 {
            let status = trailer_to_status(raw, resp.grpc_message);
            let err = grpc_status_to_app_error(status);
            return TranscodedResponse::Problem(app_error_to_problem(&err));
        }
    }

    // 2. Unframe.
    let proto_bytes = match unframe(resp.body) {
        Ok(b) => b,
        Err(e) => {
            return TranscodedResponse::Problem(app_error_to_problem(&e));
        }
    };

    transcode_unframed(resp.method_desc, proto_bytes, resp.success_status)
}

/// Transcode an already-unframed proto payload (e.g. what tonic's
/// [`tonic::client::Grpc::unary`] produces with a byte-passthrough codec)
/// into the client-facing JSON response.
pub fn transcode_unframed(
    method_desc: &MethodDescriptor,
    proto_bytes: &[u8],
    success_status: u16,
) -> TranscodedResponse {
    let msg = match DynamicMessage::decode(method_desc.output(), Bytes::copy_from_slice(proto_bytes)) {
        Ok(m) => m,
        Err(e) => {
            let err = AppError::Serialization {
                message: format!("proto decode: {e}"),
            };
            return TranscodedResponse::Problem(app_error_to_problem(&err));
        }
    };

    // Serialise to JSON (camelCase, explicit nulls for absent optionals).
    let opts = SerializeOptions::new()
        .use_proto_field_name(false)
        .stringify_64_bit_integers(true)
        .skip_default_fields(false);

    let mut buf = Vec::with_capacity(proto_bytes.len() * 2);
    let mut ser = serde_json::Serializer::new(&mut buf);
    if let Err(e) = msg.serialize_with_options(&mut ser, &opts) {
        let err = AppError::Serialization {
            message: format!("proto → JSON: {e}"),
        };
        return TranscodedResponse::Problem(app_error_to_problem(&err));
    }

    // `prost-reflect` omits absent optional message fields entirely. Per the
    // BFF design, partial-failure shape is `"field": null`; walk the
    // descriptor and inject nulls for any singular message field that is not
    // present.
    let body = match serde_json::from_slice::<serde_json::Value>(&buf) {
        Ok(mut v) => {
            inject_nulls(&msg, &mut v);
            serde_json::to_vec(&v).unwrap_or(buf)
        }
        Err(_) => buf,
    };

    TranscodedResponse::Json {
        body,
        status: success_status,
    }
}

/// Walk a [`DynamicMessage`] alongside its serialised JSON representation
/// and insert `null` for any singular message field that is absent in the
/// proto.
fn inject_nulls(msg: &DynamicMessage, value: &mut serde_json::Value) {
    let serde_json::Value::Object(obj) = value else {
        return;
    };
    let desc = msg.descriptor();
    for field in desc.fields() {
        let kind = field.kind();
        let is_message = matches!(kind, Kind::Message(_));
        let is_singular = field.cardinality() != Cardinality::Repeated && !field.is_map();
        if !is_message || !is_singular {
            continue;
        }
        let json_key = field.json_name().to_owned();
        if msg.has_field(&field) {
            if let Some(inner_value) = obj.get_mut(&json_key) {
                let cow = msg.get_field(&field);
                if let Some(inner_msg) = cow.as_message() {
                    inject_nulls(inner_msg, inner_value);
                }
            }
        } else if !obj.contains_key(&json_key) {
            obj.insert(json_key, serde_json::Value::Null);
        }
    }
}

/// Strip the 5-byte gRPC length-prefix frame and return the proto bytes.
pub fn unframe(bytes: &[u8]) -> Result<&[u8], AppError> {
    if bytes.len() < 5 {
        return Err(AppError::Serialization {
            message: "framed gRPC payload shorter than 5 bytes".into(),
        });
    }
    let flag = bytes[0];
    if flag != 0 {
        return Err(AppError::Serialization {
            message: format!("unsupported gRPC compression flag: {flag}"),
        });
    }
    let len = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;
    if bytes.len() < 5 + len {
        return Err(AppError::Serialization {
            message: format!(
                "framed gRPC payload truncated: header={len}, actual={}",
                bytes.len() - 5
            ),
        });
    }
    Ok(&bytes[5..5 + len])
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

    /// Frame proto bytes with the 5-byte gRPC header.
    fn frame(proto: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(5 + proto.len());
        out.push(0);
        out.extend_from_slice(&(proto.len() as u32).to_be_bytes());
        out.extend_from_slice(proto);
        out
    }

    #[test]
    fn success_unary_serialises_json() {
        // Build a concrete response proto with the compiled fixture types.
        let resp = proto::EchoResponse {
            message: "hello".into(),
            code: 42,
        };
        let mut proto_bytes = Vec::new();
        resp.encode(&mut proto_bytes).unwrap();
        let framed = frame(&proto_bytes);

        let md = method_desc("fixture.v1.FixtureService", "Echo");
        let out = transcode(TranscodeResponse {
            method_desc: &md,
            body: &framed,
            grpc_status: Some("0"),
            grpc_message: None,
            success_status: 200,
        });

        match out {
            TranscodedResponse::Json { body, status } => {
                assert_eq!(status, 200);
                let text = std::str::from_utf8(&body).unwrap();
                assert!(text.contains("\"message\":\"hello\""), "got: {text}");
                assert!(text.contains("\"code\":42"), "got: {text}");
            }
            other => panic!("expected Json, got {other:?}"),
        }
    }

    #[test]
    fn non_zero_grpc_status_maps_to_problem() {
        let md = method_desc("fixture.v1.FixtureService", "Echo");
        let out = transcode(TranscodeResponse {
            method_desc: &md,
            body: &frame(&[]),
            grpc_status: Some("5"), // NOT_FOUND
            grpc_message: Some("order absent"),
            success_status: 200,
        });
        match out {
            TranscodedResponse::Problem(pd) => {
                assert_eq!(pd.status, 404);
                assert!(pd.problem_type.contains("not-found"));
                assert!(pd.detail.contains("order absent"));
            }
            other => panic!("expected Problem, got {other:?}"),
        }
    }

    #[test]
    fn missing_frame_header_returns_problem() {
        let md = method_desc("fixture.v1.FixtureService", "Echo");
        let out = transcode(TranscodeResponse {
            method_desc: &md,
            body: &[0u8, 0, 0], // too short
            grpc_status: Some("0"),
            grpc_message: None,
            success_status: 200,
        });
        match out {
            TranscodedResponse::Problem(pd) => {
                assert_eq!(pd.status, 500);
            }
            other => panic!("expected Problem, got {other:?}"),
        }
    }
}
