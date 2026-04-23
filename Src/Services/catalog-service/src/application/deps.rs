use std::sync::Arc;

use ddd_shared_kernel::storage::BlobStorage;

use crate::domain::repositories::*;

pub struct AppDeps {
    pub product_repo:  Arc<dyn ProductRepository>,
    pub category_repo: Arc<dyn CategoryRepository>,
    pub brand_repo:    Arc<dyn BrandRepository>,
    pub tax_repo:      Arc<dyn TaxConfigRepository>,
    pub blob_storage:  Arc<dyn BlobStorage>,
    pub blob_bucket:   String,
    pub presign_ttl_secs: u64,
}
