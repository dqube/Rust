//! REST request → gRPC-framed proto bytes.
//!
//! Given a compiled gRPC method descriptor, a set of captured path
//! parameters, query string, headers, a body spec, and a JSON body, this
//! module:
//!
//! 1. Deserialises the JSON body (honouring `body: "*"` or `body: <field>`).
//! 2. Applies path/query/header bindings with string→scalar coercion.
//! 3. Encodes the resulting [`DynamicMessage`] to proto wire bytes.
//! 4. Prepends the 5-byte gRPC length-prefix frame.
//!
//! The result is a [`FramedRequest`] ready to ship as the HTTP/2 request
//! body to the upstream gRPC endpoint.

use std::collections::HashMap;

use bytes::{BufMut, Bytes, BytesMut};
use ddd_shared_kernel::{AppError, AppResult};
use http::HeaderMap;
use prost::Message;
use prost_reflect::{DynamicMessage, FieldDescriptor, Kind, MethodDescriptor, Value};
use crate::edge::route_config::{BindingSource, GrpcTarget};

/// Inputs to the request transcoder.
pub struct TranscodeRequest<'a> {
    /// Resolved gRPC method descriptor.
    pub method_desc: &'a MethodDescriptor,
    /// Route's gRPC target (bindings + body spec + service/method names).
    pub grpc: &'a GrpcTarget,
    /// Path parameters captured by the router.
    pub path_params: &'a HashMap<String, String>,
    /// URL query string as a key→value map.
    pub query: &'a HashMap<String, String>,
    /// Request headers.
    pub headers: &'a HeaderMap,
    /// Raw JSON request body (may be empty for GET etc.).
    pub body: &'a [u8],
}

/// Encoded (unframed) gRPC request bytes and target pseudo-path. Callers
/// dispatching via `tonic::client::Grpc` (which handles framing) should use
/// this; callers writing raw HTTP/2 bodies should call [`frame`] on the
/// [`Encoded::proto_bytes`].
pub struct Encoded {
    /// Unframed, proto-encoded payload (no 5-byte gRPC prefix).
    pub proto_bytes: Bytes,
    /// `:path` header value for the upstream HTTP/2 request
    /// (`/<service>/<method>`).
    pub grpc_path: String,
}

/// Framed gRPC request bytes and target pseudo-path.
pub struct FramedRequest {
    /// 5-byte framed, proto-encoded payload.
    pub bytes: Bytes,
    /// `:path` header value for the upstream HTTP/2 request
    /// (`/<service>/<method>`).
    pub grpc_path: String,
}

/// Prepend the 5-byte gRPC length-prefix frame to a proto-encoded payload.
pub fn frame(proto_bytes: &[u8]) -> Bytes {
    let mut framed = BytesMut::with_capacity(5 + proto_bytes.len());
    framed.put_u8(0); // compression flag (uncompressed)
    framed.put_u32(proto_bytes.len() as u32); // big-endian length
    framed.extend_from_slice(proto_bytes);
    framed.freeze()
}

/// Encode a [`TranscodeRequest`] into unframed proto bytes + the gRPC path.
/// For a framed payload, use [`transcode`] instead.
pub fn encode(req: TranscodeRequest<'_>) -> AppResult<Encoded> {
    let input_desc = req.method_desc.input();
    let mut msg = DynamicMessage::new(input_desc.clone());

    // 1. Body binding (applies first so path/query/header can override).
    if let Some(body_spec) = &req.grpc.body {
        if !req.body.is_empty() {
            if body_spec == "*" {
                let mut de = serde_json::Deserializer::from_slice(req.body);
                msg = DynamicMessage::deserialize(input_desc.clone(), &mut de).map_err(|e| {
                    AppError::Serialization {
                        message: format!("JSON body → proto: {e}"),
                    }
                })?;
            } else {
                let field = input_desc.get_field_by_name(body_spec).ok_or_else(|| {
                    AppError::Serialization {
                        message: format!(
                            "body field `{body_spec}` not found on `{}`",
                            input_desc.full_name()
                        ),
                    }
                })?;
                match field.kind() {
                    Kind::Message(sub_desc) => {
                        let mut de = serde_json::Deserializer::from_slice(req.body);
                        let sub = DynamicMessage::deserialize(sub_desc, &mut de).map_err(|e| {
                            AppError::Serialization {
                                message: format!("JSON body → proto field `{body_spec}`: {e}"),
                            }
                        })?;
                        msg.set_field(&field, Value::Message(sub));
                    }
                    _ => {
                        return Err(AppError::Serialization {
                            message: format!(
                                "body field `{body_spec}` is not a message; only message-typed \
                                 fields can receive a JSON body"
                            ),
                        });
                    }
                }
            }
        }
    }

    // 2. Bindings in declaration order.
    for b in &req.grpc.bindings {
        let src = b.source()?;
        if matches!(src, BindingSource::Body) {
            // `body` source on a binding is invalid — body handled above.
            return Err(AppError::Serialization {
                message: "binding `from: body` is not supported; use the top-level \
                          `body:` field on the gRPC target"
                    .into(),
            });
        }
        let field = input_desc.get_field_by_name(&b.to).ok_or_else(|| {
            AppError::Serialization {
                message: format!(
                    "binding target `{}` not found on `{}`",
                    b.to,
                    input_desc.full_name()
                ),
            }
        })?;

        let raw = match src {
            BindingSource::Path(name) => req.path_params.get(&name).cloned(),
            BindingSource::Query(name) => req.query.get(&name).cloned(),
            BindingSource::Header(name) => req
                .headers
                .get(&name)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_owned()),
            BindingSource::Body => unreachable!(),
        };

        if let Some(s) = raw {
            set_scalar_from_string(&mut msg, &field, &s)?;
        }
    }

    // 3. Encode.
    let mut proto_bytes = Vec::with_capacity(msg.encoded_len());
    msg.encode(&mut proto_bytes)
        .map_err(|e| AppError::Serialization {
            message: format!("proto encode: {e}"),
        })?;

    Ok(Encoded {
        proto_bytes: Bytes::from(proto_bytes),
        grpc_path: format!("/{}/{}", req.grpc.service, req.grpc.method),
    })
}

/// Build a framed gRPC request from the transcoder inputs.
///
/// For dispatching through `tonic::client::Grpc` (which frames internally)
/// call [`encode`] instead.
pub fn transcode(req: TranscodeRequest<'_>) -> AppResult<FramedRequest> {
    let Encoded {
        proto_bytes,
        grpc_path,
    } = encode(req)?;
    Ok(FramedRequest {
        bytes: frame(&proto_bytes),
        grpc_path,
    })
}

/// Coerce a string (path/query/header value) into the field's scalar type.
fn set_scalar_from_string(
    msg: &mut DynamicMessage,
    field: &FieldDescriptor,
    s: &str,
) -> AppResult<()> {
    let value = match field.kind() {
        Kind::String => Value::String(s.to_owned()),
        Kind::Bool => Value::Bool(parse_or_err(s, field, "bool")?),
        Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => {
            Value::I32(parse_or_err(s, field, "i32")?)
        }
        Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => {
            Value::I64(parse_or_err(s, field, "i64")?)
        }
        Kind::Uint32 | Kind::Fixed32 => Value::U32(parse_or_err(s, field, "u32")?),
        Kind::Uint64 | Kind::Fixed64 => Value::U64(parse_or_err(s, field, "u64")?),
        Kind::Float => Value::F32(parse_or_err(s, field, "f32")?),
        Kind::Double => Value::F64(parse_or_err(s, field, "f64")?),
        Kind::Bytes => Value::Bytes(Bytes::copy_from_slice(s.as_bytes())),
        Kind::Enum(enum_desc) => {
            let number = enum_desc
                .get_value_by_name(s)
                .map(|v| v.number())
                .or_else(|| s.parse::<i32>().ok())
                .ok_or_else(|| AppError::Serialization {
                    message: format!(
                        "field `{}`: unknown enum value `{s}` for `{}`",
                        field.name(),
                        enum_desc.full_name()
                    ),
                })?;
            Value::EnumNumber(number)
        }
        Kind::Message(_) => {
            return Err(AppError::Serialization {
                message: format!(
                    "field `{}`: path/query/header bindings cannot target message fields",
                    field.name()
                ),
            });
        }
    };
    msg.set_field(field, value);
    Ok(())
}

fn parse_or_err<T: std::str::FromStr>(
    s: &str,
    field: &FieldDescriptor,
    type_name: &str,
) -> AppResult<T>
where
    T::Err: std::fmt::Display,
{
    s.parse::<T>().map_err(|e| AppError::Serialization {
        message: format!(
            "field `{}`: expected {type_name}, got `{s}`: {e}",
            field.name()
        ),
    })
}

/// Parse a URL query string into a flat key→value map (last-wins).
pub fn parse_query(raw: Option<&str>) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Some(q) = raw else {
        return out;
    };
    for pair in q.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        out.insert(
            percent_decode(k).unwrap_or_else(|| k.to_owned()),
            percent_decode(v).unwrap_or_else(|| v.to_owned()),
        );
    }
    out
}

fn percent_decode(s: &str) -> Option<String> {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = hex(bytes[i + 1])?;
                let lo = hex(bytes[i + 2])?;
                out.push((hi << 4 | lo) as char);
                i += 3;
            }
            c if c.is_ascii() => {
                out.push(c as char);
                i += 1;
            }
            _ => return None,
        }
    }
    Some(out)
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Parse the framing of a gRPC payload: returns the unframed proto bytes.
/// Exposed here for tests of the whole round-trip.
#[cfg(test)]
pub(crate) fn unframe(bytes: &[u8]) -> AppResult<&[u8]> {
    if bytes.len() < 5 {
        return Err(AppError::Serialization {
            message: "framed payload shorter than 5 bytes".into(),
        });
    }
    let compression = bytes[0];
    if compression != 0 {
        return Err(AppError::Serialization {
            message: format!("unexpected compression flag: {compression}"),
        });
    }
    let len = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;
    if bytes.len() != 5 + len {
        return Err(AppError::Serialization {
            message: format!(
                "framed payload length mismatch: header says {len}, got {}",
                bytes.len() - 5
            ),
        });
    }
    Ok(&bytes[5..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge::route_config::{Binding, GrpcTarget};
    use crate::transcode;
    use http::HeaderMap;

    fn method_desc(service: &str, method: &str) -> prost_reflect::MethodDescriptor {
        let pool = transcode::load().unwrap();
        pool.get_service_by_name(service)
            .and_then(|s| s.methods().find(|m| m.name() == method))
            .expect("method exists")
    }

    #[test]
    fn binds_path_param_to_string_field() {
        let md = method_desc("fixture.v1.FixtureService", "Echo");
        let grpc = GrpcTarget {
            service: "fixture.v1.FixtureService".to_owned(),
            method: "Echo".to_owned(),
            bindings: vec![Binding {
                from: "path.message".to_owned(),
                to: "message".to_owned(),
            }],
            body: None,
        };
        let mut params = HashMap::new();
        params.insert("message".to_owned(), "hello-42".to_owned());
        let headers = HeaderMap::new();
        let query = HashMap::new();

        let framed = transcode(TranscodeRequest {
            method_desc: &md,
            grpc: &grpc,
            path_params: &params,
            query: &query,
            headers: &headers,
            body: &[],
        })
        .expect("transcodes");

        assert_eq!(framed.grpc_path, "/fixture.v1.FixtureService/Echo");

        let proto_bytes = unframe(&framed.bytes).unwrap();
        let input_desc = md.input();
        let decoded = DynamicMessage::decode(input_desc, proto_bytes).unwrap();
        let message = decoded
            .get_field_by_name("message")
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(message, "hello-42");
    }

    #[test]
    fn body_star_merges_json_root() {
        // EchoRequest has `string message = 1;` — ideal for body binding tests.
        let md = method_desc("fixture.v1.FixtureService", "Echo");
        let grpc = GrpcTarget {
            service: "fixture.v1.FixtureService".to_owned(),
            method: "Echo".to_owned(),
            bindings: vec![],
            body: Some("*".to_owned()),
        };
        let params = HashMap::new();
        let query = HashMap::new();
        let headers = HeaderMap::new();
        let body = br#"{"message":"from-body"}"#;

        let framed = transcode(TranscodeRequest {
            method_desc: &md,
            grpc: &grpc,
            path_params: &params,
            query: &query,
            headers: &headers,
            body,
        })
        .expect("transcodes");

        let proto_bytes = unframe(&framed.bytes).unwrap();
        let decoded = DynamicMessage::decode(md.input(), proto_bytes).unwrap();
        assert_eq!(
            decoded
                .get_field_by_name("message")
                .unwrap()
                .as_str()
                .unwrap(),
            "from-body"
        );
    }

    #[test]
    fn path_param_overrides_body() {
        let md = method_desc("fixture.v1.FixtureService", "Echo");
        let grpc = GrpcTarget {
            service: "fixture.v1.FixtureService".to_owned(),
            method: "Echo".to_owned(),
            bindings: vec![Binding {
                from: "path.message".to_owned(),
                to: "message".to_owned(),
            }],
            body: Some("*".to_owned()),
        };
        let mut params = HashMap::new();
        params.insert("message".to_owned(), "from-path".to_owned());
        let query = HashMap::new();
        let headers = HeaderMap::new();
        let body = br#"{"message":"from-body"}"#;

        let framed = transcode(TranscodeRequest {
            method_desc: &md,
            grpc: &grpc,
            path_params: &params,
            query: &query,
            headers: &headers,
            body,
        })
        .unwrap();

        let decoded = DynamicMessage::decode(md.input(), unframe(&framed.bytes).unwrap()).unwrap();
        assert_eq!(
            decoded.get_field_by_name("message").unwrap().as_str().unwrap(),
            "from-path"
        );
    }

    #[test]
    fn framing_is_5_bytes_then_payload() {
        let md = method_desc("fixture.v1.FixtureService", "Echo");
        let grpc = GrpcTarget {
            service: "fixture.v1.FixtureService".to_owned(),
            method: "Echo".to_owned(),
            bindings: vec![],
            body: None,
        };
        let framed = transcode(TranscodeRequest {
            method_desc: &md,
            grpc: &grpc,
            path_params: &HashMap::new(),
            query: &HashMap::new(),
            headers: &HeaderMap::new(),
            body: &[],
        })
        .unwrap();

        assert!(framed.bytes.len() >= 5);
        assert_eq!(framed.bytes[0], 0, "compression flag is 0");
        let len = u32::from_be_bytes([
            framed.bytes[1],
            framed.bytes[2],
            framed.bytes[3],
            framed.bytes[4],
        ]) as usize;
        assert_eq!(len, framed.bytes.len() - 5);
    }

    #[test]
    fn parse_query_basic() {
        let q = parse_query(Some("a=1&b=hello&c=%20space&d=a+b"));
        assert_eq!(q["a"], "1");
        assert_eq!(q["b"], "hello");
        assert_eq!(q["c"], " space");
        assert_eq!(q["d"], "a b");
    }

    #[test]
    fn parse_query_empty_and_none() {
        assert!(parse_query(None).is_empty());
        assert!(parse_query(Some("")).is_empty());
    }
}
