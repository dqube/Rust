use std::sync::Arc;

use ddd_shared_kernel::storage::BlobStorage;

use crate::domain::repositories::*;

pub struct AppDeps {
    pub supplier_repo:  Arc<dyn SupplierRepository>,
    pub address_repo:   Arc<dyn SupplierAddressRepository>,
    pub contact_repo:   Arc<dyn SupplierContactRepository>,
    pub document_repo:  Arc<dyn SupplierDocumentRepository>,
    pub product_repo:   Arc<dyn SupplierProductRepository>,
    pub order_repo:     Arc<dyn PurchaseOrderRepository>,
    pub blob_storage:   Arc<dyn BlobStorage>,
    pub blob_bucket:    String,
    pub presign_ttl_secs: u64,
}
