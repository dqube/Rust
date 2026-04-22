//! Blob / object-storage port (secondary port / driven adapter).
//!
//! Services depend on [`BlobStorage`] to upload, download, delete, and mint
//! presigned URLs against an object store.  Concrete adapters (AWS S3,
//! MinIO, SeaweedFS, Azure Blob) live in `ddd-infrastructure` behind crate
//! features so heavy SDK dependencies do not leak into the domain.
//!
//! # Design choices
//!
//! - [`bytes::Bytes`] on the wire — cheap clone, compatible with every
//!   mainstream SDK (aws-sdk-s3, azure-storage-blobs, google-cloud-storage).
//! - Explicit `content_type` on uploads and presigned PUTs — presigned S3
//!   signatures bind the `Content-Type` header, so callers must see it as
//!   part of the contract.
//! - `AppError::NotFound` for missing keys — adapters are expected to map
//!   provider-specific "no such key" errors to [`AppError::not_found`].
//!
//! [`AppError::not_found`]: crate::AppError::not_found

use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};

use crate::AppResult;

/// A short-lived, provider-signed URL that can be handed to a browser or
/// mobile client for a direct upload / download — the bytes bypass the
/// service.
#[derive(Debug, Clone)]
pub struct PresignedUrl {
    /// Fully-qualified URL including query-string signature.
    pub url: String,
    /// When the signature stops being valid.  Callers typically treat
    /// 5–10 seconds before this as the effective deadline.
    pub expires_at: DateTime<Utc>,
}

impl PresignedUrl {
    /// Build a `PresignedUrl` from a URL and a TTL measured from *now*.
    pub fn new(url: impl Into<String>, ttl: Duration) -> Self {
        let expires_at = Utc::now()
            + chrono::Duration::from_std(ttl).unwrap_or(chrono::Duration::zero());
        Self { url: url.into(), expires_at }
    }
}

/// Port that every object-storage adapter must implement.
///
/// Implementations are `Send + Sync` and typically `Clone`; construct once
/// per service and share an `Arc<dyn BlobStorage>` across handlers.
#[async_trait]
pub trait BlobStorage: Send + Sync {
    /// Upload `data` to `{bucket}/{key}` with the given `content_type`.
    ///
    /// # Errors
    /// Returns [`AppError::internal`](crate::AppError::internal) for
    /// transport / auth failures and the adapter's natural error kind for
    /// bucket-level issues (missing bucket, permission denied, etc.).
    async fn upload(
        &self,
        bucket: &str,
        key: &str,
        content_type: &str,
        data: Bytes,
    ) -> AppResult<()>;

    /// Download the object at `{bucket}/{key}`.
    ///
    /// # Errors
    /// Returns [`AppError::not_found`](crate::AppError::not_found) when the
    /// key does not exist; `internal` for every other failure.
    async fn download(&self, bucket: &str, key: &str) -> AppResult<Bytes>;

    /// Delete the object at `{bucket}/{key}`.  Idempotent — deleting a
    /// missing key succeeds.
    async fn delete(&self, bucket: &str, key: &str) -> AppResult<()>;

    /// Generate a presigned URL that the client may use to `PUT` an object
    /// directly.  The client **must** send `Content-Type: {content_type}`
    /// matching the value signed here or the upload will be rejected.
    async fn presigned_put(
        &self,
        bucket: &str,
        key: &str,
        content_type: &str,
        ttl: Duration,
    ) -> AppResult<PresignedUrl>;

    /// Generate a presigned URL that the client may use to `GET` an object
    /// directly.
    async fn presigned_get(
        &self,
        bucket: &str,
        key: &str,
        ttl: Duration,
    ) -> AppResult<PresignedUrl>;
}
