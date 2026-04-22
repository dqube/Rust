use std::sync::Arc;

use ddd_shared_kernel::BlobStorage;

use crate::domain::repositories::{
    CustomerProfileRepository, CustomerRepository, WishlistItemRepository,
};

/// Dependency container handed to inventory-registered handler factories.
#[derive(Clone)]
pub struct AppDeps {
    pub customer_repo: Arc<dyn CustomerRepository>,
    pub profile_repo: Arc<dyn CustomerProfileRepository>,
    pub wishlist_repo: Arc<dyn WishlistItemRepository>,
    pub blob_storage: Arc<dyn BlobStorage>,
    /// Bucket holding avatars + KYC documents.
    pub blob_bucket: String,
    /// Default presigned URL TTL (seconds).
    pub presign_ttl_secs: u64,
}
