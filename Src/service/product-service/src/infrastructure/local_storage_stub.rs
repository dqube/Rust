//! Stub storage adapter for local development.
//!
//! Returns a fake presigned URL pointing at localhost.
//! Replace this with an Azure Blob Storage or S3 implementation in production.

use async_trait::async_trait;
use ddd_shared_kernel::AppResult;

use crate::application::ports::StoragePort;

pub struct LocalStorageStub {
    /// Base URL prefix for generated fake URLs, e.g. "http://localhost:10000/devstoreaccount1".
    pub base_url: String,
    pub expires_in_secs: u32,
}

impl LocalStorageStub {
    pub fn new() -> Self {
        Self {
            base_url: "http://localhost:10000/devstoreaccount1/products".to_string(),
            expires_in_secs: 900,
        }
    }
}

impl Default for LocalStorageStub {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StoragePort for LocalStorageStub {
    async fn presigned_upload_url(
        &self,
        object_key: &str,
        _content_type: &str,
    ) -> AppResult<(String, u32)> {
        let url = format!(
            "{}/{}?sv=2020-08-04&se=stub&sp=w&spr=https&sig=stub",
            self.base_url, object_key
        );
        Ok((url, self.expires_in_secs))
    }
}
