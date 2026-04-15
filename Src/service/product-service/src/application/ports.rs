//! Application-layer ports (secondary ports / driven adapters).

use async_trait::async_trait;
use ddd_shared_kernel::AppResult;

/// Port for blob/object storage — generates presigned upload URLs.
///
/// In production, implement this against Azure Blob Storage or S3.
/// The in-memory stub returns a fake URL for local development.
#[async_trait]
pub trait StoragePort: Send + Sync {
    /// Generate a presigned HTTP PUT URL for the given object key.
    ///
    /// Returns `(upload_url, expires_in_secs)`.
    async fn presigned_upload_url(
        &self,
        object_key: &str,
        content_type: &str,
    ) -> AppResult<(String, u32)>;
}
