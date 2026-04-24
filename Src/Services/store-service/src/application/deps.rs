use std::sync::Arc;

use ddd_shared_kernel::{BlobStorage, OutboxRepository};

use crate::domain::repositories::{RegisterRepository, StoreRepository};

pub struct AppDeps {
    pub store_repo:    Arc<dyn StoreRepository>,
    pub register_repo: Arc<dyn RegisterRepository>,
    pub outbox:        Arc<dyn OutboxRepository>,
    pub blob_storage:  Arc<dyn BlobStorage>,
    pub blob_bucket:   String,
}
