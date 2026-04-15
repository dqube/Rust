//! gRPC building blocks: server, interceptors, error mapping, pagination,
//! streaming, validation, and proto-mapping helpers.

pub mod error;
pub mod global_error_handler;
pub mod idempotency;
pub mod interceptor;
pub mod mapper;
pub mod metadata;
pub mod pagination;
pub mod server;
pub mod streaming;
pub mod validation;

#[cfg(feature = "jwt")]
pub mod auth;

pub use error::GrpcErrorExt;
pub use global_error_handler::{error_mapping_interceptor, normalise_result, normalise_status};
pub use idempotency::extract_idempotency_key;
pub use mapper::{FromProto, IntoProto};
pub use metadata::HasMetadata;
pub use pagination::{ProtoPageInfo, proto_page_request, proto_page_response};
pub use server::GrpcServer;
pub use streaming::TonicStream;
pub use validation::{app_error_to_status, GrpcValidationExt, GrpcValidatorRegistryExt};
