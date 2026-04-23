use std::sync::Arc;

use ddd_shared_kernel::BlobStorage;
use ddd_shared_kernel::OutboxRepository;

use crate::domain::repositories::{OrderSagaRepository, ReturnRepository, SaleRepository};

pub struct AppDeps {
    pub sale_repo:        Arc<dyn SaleRepository>,
    pub return_repo:      Arc<dyn ReturnRepository>,
    pub saga_repo:        Arc<dyn OrderSagaRepository>,
    pub outbox:           Arc<dyn OutboxRepository>,
    pub blob_storage:     Arc<dyn BlobStorage>,
    pub blob_bucket:      String,
    pub presign_ttl_secs: u64,
}
