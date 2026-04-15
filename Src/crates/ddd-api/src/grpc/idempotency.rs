//! gRPC helper for extracting the `idempotency-key` metadata value.
//!
//! # Example
//! ```ignore
//! use ddd_api::grpc::idempotency::extract_idempotency_key;
//!
//! async fn create_order(req: Request<CreateOrderMsg>) -> Result<Response<OrderId>, Status> {
//!     let key = extract_idempotency_key(&req)?;
//!     // ...
//! }
//! ```

use tonic::{Request, Status};

/// The metadata key used to transmit the idempotency key in gRPC calls.
///
/// Alias for [`crate::common::error_mapping::IDEMPOTENCY_KEY`] kept for
/// backwards compatibility.
pub const IDEMPOTENCY_KEY_METADATA: &str = crate::common::error_mapping::IDEMPOTENCY_KEY;

/// Extract the `idempotency-key` metadata value from a gRPC request.
///
/// Returns `Status::invalid_argument` when the key is missing or empty.
#[allow(clippy::result_large_err)]
pub fn extract_idempotency_key<T>(req: &Request<T>) -> Result<String, Status> {
    req.metadata()
        .get(IDEMPOTENCY_KEY_METADATA)
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .ok_or_else(|| {
            Status::invalid_argument(
                "the idempotency-key metadata header is required for this request",
            )
        })
}
