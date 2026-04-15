//! Extract metadata from gRPC requests.

use tonic::Request;

/// Extension trait for extracting metadata values from a [`tonic::Request`].
pub trait HasMetadata {
    /// Retrieve a metadata value by key.
    fn get_metadata(&self, key: &str) -> Option<String>;
}

impl<T> HasMetadata for Request<T> {
    fn get_metadata(&self, key: &str) -> Option<String> {
        self.metadata()
            .get(key)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned())
    }
}

/// Extract the `authorization` header value from a gRPC request.
pub fn extract_auth<T>(req: &Request<T>) -> Option<String> {
    req.get_metadata("authorization")
}

/// Extract the `x-request-id` header value from a gRPC request.
pub fn extract_request_id<T>(req: &Request<T>) -> Option<String> {
    req.get_metadata("x-request-id")
}

/// Extract the `x-tenant-id` header value from a gRPC request.
pub fn extract_tenant_id<T>(req: &Request<T>) -> Option<String> {
    req.get_metadata("x-tenant-id")
}
