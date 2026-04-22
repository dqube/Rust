use std::sync::Arc;

use ddd_shared_kernel::BlobStorage;

use crate::domain::repository::ProductRepository;

#[derive(Clone)]
pub struct AppDeps {
    pub product_repo: Arc<dyn ProductRepository>,
    pub storage: Arc<dyn BlobStorage>,
    /// Bucket where product images are uploaded.
    pub image_bucket: String,
    /// TTL applied to every presigned PUT URL minted by this service.
    pub presign_ttl: std::time::Duration,
}
