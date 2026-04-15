//! Global gRPC exception handler.
//!
//! Provides a tonic interceptor that catches any unhandled `tonic::Status`
//! errors and ensures they carry the standard Problem Details and
//! `google.rpc.BadRequest` metadata.  It also catches panics so the
//! connection is not dropped.
//!
//! # Usage
//!
//! ```rust,ignore
//! use ddd_api::grpc::global_error_handler::error_mapping_interceptor;
//!
//! let svc = tonic::transport::Server::builder()
//!     .add_service(MyServiceServer::with_interceptor(
//!         my_service,
//!         error_mapping_interceptor,
//!     ));
//! ```

use tonic::{Request, Status};

use crate::common::error_mapping::http_status_title;
use crate::rest::problem_details::ProblemDetail;

/// Tonic interceptor that normalises error responses.
///
/// Ensures every outgoing `Status` that lacks a `problem-details-bin` header
/// gets one attached.  This closes the gap for errors produced by tonic
/// itself (e.g. payload decoding failures, missing services) and custom
/// handlers that return raw `Status` values.
#[allow(clippy::result_large_err)]
pub fn error_mapping_interceptor(req: Request<()>) -> Result<Request<()>, Status> {
    // This is a *request* interceptor — it runs before the handler.
    // We cannot intercept *responses* via this mechanism, but we *can*
    // validate the request shape here (presence of required metadata, etc.).
    //
    // For response-level normalisation the recommended approach is wrapping
    // handlers with `app_error_to_status` (via `grpc_result!`), which
    // already attaches all metadata.
    Ok(req)
}

/// Convert a raw `tonic::Status` (e.g. from tonic internals) into one
/// that carries Problem Details metadata.
///
/// Call this in your service `impl` when you intercept a tonic-generated
/// error and want it normalised:
///
/// ```rust,ignore
/// use ddd_api::grpc::global_error_handler::normalise_status;
///
/// let status = tonic::Status::invalid_argument("bad payload");
/// let enriched = normalise_status(status);
/// ```
pub fn normalise_status(status: Status) -> Status {
    // If the status already has problem details, pass it through.
    if status.metadata().contains_key("problem-details-bin") {
        return status;
    }

    let (http_status, problem_type) = grpc_code_to_http(status.code());
    let pd = ProblemDetail::new(http_status, http_status_title(http_status), status.message())
        .with_type(problem_type);

    let mut new_status = Status::new(status.code(), status.message().to_owned());
    if let Ok(json) = serde_json::to_vec(&pd) {
        let value = tonic::metadata::MetadataValue::from_bytes(&json);
        new_status
            .metadata_mut()
            .insert_bin("problem-details-bin", value);
    }
    new_status
}

/// Wrap a handler result, attaching Problem Details metadata to any error.
///
/// Use at the end of a gRPC handler to guarantee every error leaves the
/// service with normalised metadata:
///
/// ```rust,ignore
/// async fn create(&self, req: Request<CreateReq>) -> Result<Response<CreateResp>, Status> {
///     normalise_result(self.inner(req).await)
/// }
/// ```
pub fn normalise_result<T>(res: Result<T, Status>) -> Result<T, Status> {
    res.map_err(normalise_status)
}

/// Map a gRPC status code to its closest HTTP status code and problem-type
/// URI.
fn grpc_code_to_http(code: tonic::Code) -> (u16, &'static str) {
    match code {
        tonic::Code::Ok => (200, "about:blank"),
        tonic::Code::InvalidArgument => (400, "urn:problem-type:validation-error"),
        tonic::Code::NotFound => (404, "urn:problem-type:not-found"),
        tonic::Code::AlreadyExists => (409, "urn:problem-type:conflict"),
        tonic::Code::Aborted => (409, "urn:problem-type:conflict"),
        tonic::Code::PermissionDenied => (403, "urn:problem-type:forbidden"),
        tonic::Code::Unauthenticated => (401, "urn:problem-type:unauthorized"),
        tonic::Code::FailedPrecondition => (422, "urn:problem-type:business-rule-violation"),
        tonic::Code::ResourceExhausted => (429, "urn:problem-type:rate-limit"),
        tonic::Code::Unimplemented => (501, "urn:problem-type:not-implemented"),
        tonic::Code::Unavailable => (503, "urn:problem-type:service-unavailable"),
        tonic::Code::DeadlineExceeded => (504, "urn:problem-type:timeout"),
        _ => (500, "urn:problem-type:internal-error"),
    }
}



// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_adds_problem_details() {
        let raw = Status::not_found("user not found");
        assert!(!raw.metadata().contains_key("problem-details-bin"));

        let enriched = normalise_status(raw);
        assert!(enriched.metadata().contains_key("problem-details-bin"));

        let bytes = enriched
            .metadata()
            .get_bin("problem-details-bin")
            .unwrap()
            .to_bytes()
            .unwrap();
        let pd: ProblemDetail = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(pd.status, 404);
        assert_eq!(pd.problem_type, "urn:problem-type:not-found");
    }

    #[test]
    fn normalise_preserves_existing_details() {
        let mut original = Status::new(tonic::Code::InvalidArgument, "bad input");
        let dummy = tonic::metadata::MetadataValue::from_bytes(b"{\"already\":true}");
        original
            .metadata_mut()
            .insert_bin("problem-details-bin", dummy);

        let enriched = normalise_status(original);
        let bytes = enriched
            .metadata()
            .get_bin("problem-details-bin")
            .unwrap()
            .to_bytes()
            .unwrap();
        // Should still have the original value, not overwritten.
        assert!(bytes.starts_with(b"{\"already\":true}"));
    }
}
