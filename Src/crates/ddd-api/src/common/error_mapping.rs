//! Shared error-mapping helpers used by both the gRPC and REST modules.

// ─── Idempotency key ─────────────────────────────────────────────────────────

/// The header / metadata key used to transmit the idempotency key.
///
/// Both the REST extractor ([`crate::rest::IdempotencyKey`]) and the gRPC
/// helper ([`crate::grpc::extract_idempotency_key`]) use this same value so
/// clients send a single `idempotency-key` field regardless of transport.
pub const IDEMPOTENCY_KEY: &str = "idempotency-key";

// ─── HTTP status helpers ─────────────────────────────────────────────────────

/// Return a short human-readable title for a well-known HTTP status code.
///
/// Used by both `rest::problem_details` and `grpc::global_error_handler` when
/// building RFC 9457 `ProblemDetail` objects so both transports produce
/// consistent `title` values.
pub fn http_status_title(code: u16) -> &'static str {
    match code {
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idempotency_key_value() {
        assert_eq!(IDEMPOTENCY_KEY, "idempotency-key");
    }

    #[test]
    fn status_titles_known_codes() {
        assert_eq!(http_status_title(400), "Bad Request");
        assert_eq!(http_status_title(404), "Not Found");
        assert_eq!(http_status_title(500), "Internal Server Error");
    }

    #[test]
    fn status_title_unknown_falls_back_to_error() {
        assert_eq!(http_status_title(418), "Error");
    }
}
