//! [`BlobStorage`] adapter over the AWS S3 SDK.
//!
//! Because every major S3-compatible server (MinIO, SeaweedFS, Cloudflare R2)
//! implements the same REST API, a single adapter covers them all.  The
//! relevant knobs are [`S3Config::endpoint`] (point at the non-AWS server)
//! and [`S3Config::force_path_style`] (required for MinIO / SeaweedFS, off
//! for AWS).
//!
//! # Example
//!
//! ```no_run
//! # use bytes::Bytes;
//! # use ddd_infrastructure::storage::{S3BlobStorage, S3Config};
//! # use ddd_shared_kernel::BlobStorage;
//! # async fn run() -> ddd_shared_kernel::AppResult<()> {
//! let storage = S3BlobStorage::connect(S3Config {
//!     endpoint: Some("http://localhost:9000".into()), // MinIO
//!     region: "us-east-1".into(),
//!     force_path_style: true,
//!     access_key_id: Some("minio".into()),
//!     secret_access_key: Some("minio123".into()),
//!     session_token: None,
//! })
//! .await?;
//!
//! storage
//!     .upload("media", "hero.jpg", "image/jpeg", Bytes::from_static(b"..."))
//!     .await?;
//! # Ok(()) }
//! ```

use std::time::Duration;

use async_trait::async_trait;
use aws_sdk_s3::{
    error::ProvideErrorMetadata,
    operation::get_object::GetObjectError,
    presigning::PresigningConfig,
    primitives::ByteStream,
    Client,
};
use bytes::Bytes;
use ddd_shared_kernel::{AppError, AppResult, BlobStorage, PresignedUrl};

/// Runtime configuration for [`S3BlobStorage::connect`].
///
/// Any field left `None` falls back to the AWS default credential / region
/// chain (`AWS_*` env vars, `~/.aws/credentials`, IMDS on EC2, etc.).
#[derive(Debug, Clone)]
pub struct S3Config {
    /// Override the service endpoint — set this to your MinIO / SeaweedFS
    /// URL; leave `None` to use the real AWS S3 endpoint for `region`.
    pub endpoint: Option<String>,
    /// AWS region or any label your S3-compatible server accepts
    /// (e.g. MinIO typically uses `"us-east-1"`).
    pub region: String,
    /// `true` forces path-style addressing (`http://host/bucket/key`) —
    /// required for MinIO, SeaweedFS, and most on-prem S3 servers.  `false`
    /// uses virtual-hosted style (`https://bucket.s3.amazonaws.com/key`).
    pub force_path_style: bool,
    /// Optional static access key — when set together with
    /// [`Self::secret_access_key`] the default credential chain is bypassed.
    pub access_key_id: Option<String>,
    /// Optional static secret key.
    pub secret_access_key: Option<String>,
    /// Optional session token (for AWS STS temporary credentials).
    pub session_token: Option<String>,
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            endpoint: None,
            region: "us-east-1".into(),
            force_path_style: false,
            access_key_id: None,
            secret_access_key: None,
            session_token: None,
        }
    }
}

/// S3-compatible [`BlobStorage`] implementation.
#[derive(Debug, Clone)]
pub struct S3BlobStorage {
    client: Client,
}

impl S3BlobStorage {
    /// Wrap an already-built `aws_sdk_s3::Client` — useful for tests and for
    /// services that already own an SDK config.
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Build an SDK config from the given [`S3Config`] and return a ready
    /// adapter.
    ///
    /// # Errors
    /// Returns [`AppError::internal`] when the SDK rejects the configuration.
    pub async fn connect(config: S3Config) -> AppResult<Self> {
        let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(config.region.clone()));

        if let Some(endpoint) = &config.endpoint {
            loader = loader.endpoint_url(endpoint);
        }

        if let (Some(access_key), Some(secret_key)) =
            (&config.access_key_id, &config.secret_access_key)
        {
            let creds = aws_credential_types::Credentials::new(
                access_key,
                secret_key,
                config.session_token.clone(),
                None,
                "ddd-infrastructure",
            );
            loader = loader.credentials_provider(creds);
        }

        let sdk_config = loader.load().await;

        let s3_config = aws_sdk_s3::config::Builder::from(&sdk_config)
            .force_path_style(config.force_path_style)
            .build();

        Ok(Self { client: Client::from_conf(s3_config) })
    }

    /// Underlying SDK client — exposed for advanced usage (multipart uploads,
    /// bucket management) that the port does not cover.
    pub fn client(&self) -> &Client {
        &self.client
    }
}

#[async_trait]
impl BlobStorage for S3BlobStorage {
    async fn upload(
        &self,
        bucket: &str,
        key: &str,
        content_type: &str,
        data: Bytes,
    ) -> AppResult<()> {
        self.client
            .put_object()
            .bucket(bucket)
            .key(key)
            .content_type(content_type)
            .body(ByteStream::from(data))
            .send()
            .await
            .map_err(|e| AppError::internal(format!("s3 put_object: {e}")))?;
        Ok(())
    }

    async fn download(&self, bucket: &str, key: &str) -> AppResult<Bytes> {
        let resp = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|sdk_err| match sdk_err.into_service_error() {
                GetObjectError::NoSuchKey(_) => {
                    AppError::not_found("blob", format!("{bucket}/{key}"))
                }
                other => {
                    let code = other.code().unwrap_or("").to_owned();
                    if code == "NotFound" || code == "NoSuchKey" {
                        AppError::not_found("blob", format!("{bucket}/{key}"))
                    } else {
                        AppError::internal(format!("s3 get_object: {other}"))
                    }
                }
            })?;

        let collected = resp
            .body
            .collect()
            .await
            .map_err(|e| AppError::internal(format!("s3 get_object body: {e}")))?;
        Ok(collected.into_bytes())
    }

    async fn delete(&self, bucket: &str, key: &str) -> AppResult<()> {
        self.client
            .delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("s3 delete_object: {e}")))?;
        Ok(())
    }

    async fn presigned_put(
        &self,
        bucket: &str,
        key: &str,
        content_type: &str,
        ttl: Duration,
    ) -> AppResult<PresignedUrl> {
        let cfg = PresigningConfig::expires_in(ttl)
            .map_err(|e| AppError::internal(format!("presign config: {e}")))?;
        let presigned = self
            .client
            .put_object()
            .bucket(bucket)
            .key(key)
            .content_type(content_type)
            .presigned(cfg)
            .await
            .map_err(|e| AppError::internal(format!("s3 presign put: {e}")))?;
        Ok(PresignedUrl::new(presigned.uri().to_owned(), ttl))
    }

    async fn presigned_get(
        &self,
        bucket: &str,
        key: &str,
        ttl: Duration,
    ) -> AppResult<PresignedUrl> {
        let cfg = PresigningConfig::expires_in(ttl)
            .map_err(|e| AppError::internal(format!("presign config: {e}")))?;
        let presigned = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .presigned(cfg)
            .await
            .map_err(|e| AppError::internal(format!("s3 presign get: {e}")))?;
        Ok(PresignedUrl::new(presigned.uri().to_owned(), ttl))
    }
}
