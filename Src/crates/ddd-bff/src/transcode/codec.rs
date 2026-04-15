//! Byte-passthrough [`tonic::codec::Codec`].
//!
//! Enables calling arbitrary gRPC methods from a generic
//! [`tonic::client::Grpc`] handle without knowing the concrete prost-generated
//! message types at compile time. The codec encodes/decodes raw proto bytes
//! as [`Bytes`] — the framing layer is still handled by tonic, so the edge
//! passes in *unframed* proto bytes and receives *unframed* proto bytes back.

use bytes::{Buf, BufMut, Bytes};
use tonic::{
    codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder},
    Status,
};

/// Tonic codec that passes proto bytes through without decoding.
#[derive(Debug, Clone, Default)]
pub struct BytesCodec;

/// Encoder half of [`BytesCodec`].
#[derive(Debug, Clone, Default)]
pub struct BytesEncoder;

/// Decoder half of [`BytesCodec`].
#[derive(Debug, Clone, Default)]
pub struct BytesDecoder;

impl Codec for BytesCodec {
    type Encode = Bytes;
    type Decode = Bytes;
    type Encoder = BytesEncoder;
    type Decoder = BytesDecoder;

    fn encoder(&mut self) -> Self::Encoder {
        BytesEncoder
    }

    fn decoder(&mut self) -> Self::Decoder {
        BytesDecoder
    }
}

impl Encoder for BytesEncoder {
    type Item = Bytes;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        dst.put(item);
        Ok(())
    }
}

impl Decoder for BytesDecoder {
    type Item = Bytes;
    type Error = Status;

    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        if !src.has_remaining() {
            return Ok(None);
        }
        Ok(Some(src.copy_to_bytes(src.remaining())))
    }
}
