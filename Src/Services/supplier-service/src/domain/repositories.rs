use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ddd_shared_kernel::{AppError, Page, PageRequest};
use uuid::Uuid;

use super::entities::{
    PurchaseOrder, Supplier, SupplierAddress, SupplierContact, SupplierDocument, SupplierProduct,
};
use super::ids::*;

#[async_trait]
pub trait SupplierRepository: Send + Sync {
    async fn find_by_id(&self, id: SupplierId) -> Result<Option<Supplier>, AppError>;
    async fn get_paged(
        &self,
        active_only: bool,
        search: Option<&str>,
        req: &PageRequest,
    ) -> Result<Page<Supplier>, AppError>;
    async fn code_exists(&self, code: &str) -> Result<bool, AppError>;
    async fn email_exists(&self, email: &str) -> Result<bool, AppError>;
    async fn save(&self, supplier: &Supplier) -> Result<(), AppError>;
    async fn delete(&self, id: SupplierId) -> Result<(), AppError>;
}

#[async_trait]
pub trait SupplierAddressRepository: Send + Sync {
    async fn find_by_id(&self, id: AddressId) -> Result<Option<SupplierAddress>, AppError>;
    async fn get_by_supplier(&self, supplier_id: SupplierId) -> Result<Vec<SupplierAddress>, AppError>;
    async fn save(&self, address: &SupplierAddress) -> Result<(), AppError>;
}

#[async_trait]
pub trait SupplierContactRepository: Send + Sync {
    async fn find_by_id(&self, id: ContactId) -> Result<Option<SupplierContact>, AppError>;
    async fn get_by_supplier(&self, supplier_id: SupplierId) -> Result<Vec<SupplierContact>, AppError>;
    async fn save(&self, contact: &SupplierContact) -> Result<(), AppError>;
}

#[async_trait]
pub trait SupplierDocumentRepository: Send + Sync {
    async fn find_by_id(&self, id: DocumentId) -> Result<Option<SupplierDocument>, AppError>;
    async fn get_by_supplier(&self, supplier_id: SupplierId) -> Result<Vec<SupplierDocument>, AppError>;
    async fn save(&self, doc: &SupplierDocument) -> Result<(), AppError>;
    async fn delete(&self, id: DocumentId) -> Result<(), AppError>;
}

#[async_trait]
pub trait SupplierProductRepository: Send + Sync {
    async fn find_by_id(&self, id: SupplierProductId) -> Result<Option<SupplierProduct>, AppError>;
    async fn get_by_supplier(&self, supplier_id: SupplierId) -> Result<Vec<SupplierProduct>, AppError>;
    async fn exists(&self, supplier_id: SupplierId, product_id: Uuid, variant_id: Option<Uuid>) -> Result<bool, AppError>;
    async fn save(&self, product: &SupplierProduct) -> Result<(), AppError>;
    async fn delete(&self, id: SupplierProductId) -> Result<(), AppError>;
}

#[async_trait]
pub trait PurchaseOrderRepository: Send + Sync {
    async fn find_by_id(&self, id: OrderId) -> Result<Option<PurchaseOrder>, AppError>;
    async fn get_filtered(
        &self,
        supplier_id: Option<SupplierId>,
        store_id: Option<i32>,
        status: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<PurchaseOrder>, AppError>;
    async fn save(&self, order: &PurchaseOrder) -> Result<(), AppError>;
}
