//! Proto ↔ domain mapping helpers.

use ddd_shared_kernel::AppResult;

/// Convert a protobuf type into a domain type.
pub trait FromProto<P>: Sized {
    /// Perform the conversion, potentially failing with an [`AppError`].
    fn from_proto(proto: P) -> AppResult<Self>;
}

/// Convert a domain type into a protobuf type.
pub trait IntoProto<P> {
    /// Perform the conversion.
    fn into_proto(self) -> P;
}

/// Convert a [`uuid::Uuid`] into a protobuf-friendly string.
pub fn uuid_to_proto(id: uuid::Uuid) -> String {
    id.to_string()
}

/// Parse a protobuf string into a [`uuid::Uuid`].
pub fn proto_to_uuid(s: &str) -> AppResult<uuid::Uuid> {
    s.parse::<uuid::Uuid>()
        .map_err(|e| ddd_shared_kernel::AppError::validation("id", format!("invalid UUID: {e}")))
}

/// Convert a [`chrono::DateTime<Utc>`] to a protobuf-style timestamp (RFC
/// 3339 string).
pub fn timestamp_to_proto(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.to_rfc3339()
}

/// Parse a protobuf timestamp string into a [`chrono::DateTime<Utc>`].
pub fn proto_to_timestamp(s: &str) -> AppResult<chrono::DateTime<chrono::Utc>> {
    s.parse::<chrono::DateTime<chrono::Utc>>()
        .map_err(|e| ddd_shared_kernel::AppError::validation("timestamp", format!("invalid timestamp: {e}")))
}

/// Generate `FromProto` and `IntoProto` implementations for simple 1:1 field
/// mappings.
///
/// # Example
/// ```ignore
/// impl_proto_mapper!(MyDomain, MyProto, {
///     id: String => uuid,
///     name: String => String,
/// });
/// ```
#[macro_export]
macro_rules! impl_proto_mapper {
    ($domain:ty, $proto:ty, { $($field:ident),* $(,)? }) => {
        impl $crate::grpc::FromProto<$proto> for $domain {
            fn from_proto(p: $proto) -> ddd_shared_kernel::AppResult<Self> {
                Ok(Self {
                    $( $field: p.$field, )*
                })
            }
        }

        impl $crate::grpc::IntoProto<$proto> for $domain {
            fn into_proto(self) -> $proto {
                $proto {
                    $( $field: self.$field, )*
                }
            }
        }
    };
}
