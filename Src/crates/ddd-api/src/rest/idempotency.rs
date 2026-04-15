//! REST extractor for the `Idempotency-Key` header.
//!
//! Use [`IdempotencyKey`] as a handler parameter to extract the header from
//! the incoming request. Returns `422 Unprocessable Entity` (ProblemDetail)
//! when the header is missing.
//!
//! # Example
//! ```ignore
//! use ddd_api::rest::IdempotencyKey;
//!
//! async fn create_order(
//!     IdempotencyKey(key): IdempotencyKey,
//!     Json(body): Json<CreateOrderDto>,
//! ) -> impl IntoResponse {
//!     // key: String — the client-supplied idempotency key
//! }
//! ```

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};

use super::problem_details::ProblemDetail;

/// The header name used to transmit the idempotency key.
///
/// Alias for [`crate::common::error_mapping::IDEMPOTENCY_KEY`] kept for
/// backwards compatibility.
pub const IDEMPOTENCY_KEY_HEADER: &str = crate::common::error_mapping::IDEMPOTENCY_KEY;

/// Axum extractor that reads the `Idempotency-Key` header.
pub struct IdempotencyKey(pub String);

impl<S> FromRequestParts<S> for IdempotencyKey
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let value = parts
            .headers
            .get(IDEMPOTENCY_KEY_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned());

        match value {
            Some(key) if !key.is_empty() => Ok(IdempotencyKey(key)),
            _ => {
                let pd = ProblemDetail::new(
                    422,
                    "Missing Idempotency Key",
                    "The Idempotency-Key header is required for this request",
                );
                Err(pd.into_response())
            }
        }
    }
}
