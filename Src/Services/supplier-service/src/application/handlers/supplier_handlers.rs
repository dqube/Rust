use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use ddd_application::{CommandHandler, QueryHandler, register_command_handler, register_query_handler};
use ddd_shared_kernel::{AppError, AppResult, BlobStorage, Page};

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::queries::*;
use crate::domain::entities::{Supplier, SupplierAddress, SupplierContact, SupplierDocument, SupplierProduct};
use crate::domain::repositories::{
    SupplierAddressRepository, SupplierContactRepository, SupplierDocumentRepository,
    SupplierProductRepository, SupplierRepository,
};

// ── CreateSupplier ────────────────────────────────────────────────────────────

pub struct CreateSupplierHandler {
    supplier_repo: Arc<dyn SupplierRepository>,
    address_repo:  Arc<dyn SupplierAddressRepository>,
    contact_repo:  Arc<dyn SupplierContactRepository>,
}

#[async_trait]
impl CommandHandler<CreateSupplier> for CreateSupplierHandler {
    async fn handle(&self, cmd: CreateSupplier) -> AppResult<Supplier> {
        let supplier = Supplier::create(
            cmd.company_name, cmd.tax_identification_number, cmd.registration_number,
            cmd.email, cmd.phone, cmd.website, cmd.business_type, cmd.notes, cmd.created_by.clone(),
        );
        self.supplier_repo.save(&supplier).await?;

        let addr = SupplierAddress::create(
            supplier.id, cmd.address_type, cmd.address_line1, cmd.address_city,
            cmd.address_postal, cmd.address_country, None, None, true, None, None,
        );
        self.address_repo.save(&addr).await?;

        let contact = SupplierContact::create(
            supplier.id, cmd.contact_type, cmd.contact_first_name, cmd.contact_last_name,
            cmd.contact_email, None, None, true, None, cmd.created_by,
        );
        self.contact_repo.save(&contact).await?;

        Ok(supplier)
    }
}

register_command_handler!(CreateSupplier, AppDeps, |d: &AppDeps| {
    CreateSupplierHandler {
        supplier_repo: d.supplier_repo.clone(),
        address_repo:  d.address_repo.clone(),
        contact_repo:  d.contact_repo.clone(),
    }
});

// ── UpdateSupplier ────────────────────────────────────────────────────────────

pub struct UpdateSupplierHandler {
    repo: Arc<dyn SupplierRepository>,
}

#[async_trait]
impl CommandHandler<UpdateSupplier> for UpdateSupplierHandler {
    async fn handle(&self, cmd: UpdateSupplier) -> AppResult<Supplier> {
        let mut s = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Supplier", cmd.id.to_string()))?;
        s.update(
            cmd.company_name, cmd.tax_identification_number, cmd.registration_number,
            cmd.email, cmd.phone, cmd.website, cmd.business_type, cmd.years_in_business,
            cmd.notes, cmd.updated_by,
        );
        self.repo.save(&s).await?;
        Ok(s)
    }
}

register_command_handler!(UpdateSupplier, AppDeps, |d: &AppDeps| {
    UpdateSupplierHandler { repo: d.supplier_repo.clone() }
});

// ── ActivateSupplier ──────────────────────────────────────────────────────────

pub struct ActivateSupplierHandler {
    repo: Arc<dyn SupplierRepository>,
}

#[async_trait]
impl CommandHandler<ActivateSupplier> for ActivateSupplierHandler {
    async fn handle(&self, cmd: ActivateSupplier) -> AppResult<Supplier> {
        let mut s = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Supplier", cmd.id.to_string()))?;
        s.activate(cmd.updated_by);
        self.repo.save(&s).await?;
        Ok(s)
    }
}

register_command_handler!(ActivateSupplier, AppDeps, |d: &AppDeps| {
    ActivateSupplierHandler { repo: d.supplier_repo.clone() }
});

// ── DeactivateSupplier ────────────────────────────────────────────────────────

pub struct DeactivateSupplierHandler {
    repo: Arc<dyn SupplierRepository>,
}

#[async_trait]
impl CommandHandler<DeactivateSupplier> for DeactivateSupplierHandler {
    async fn handle(&self, cmd: DeactivateSupplier) -> AppResult<Supplier> {
        let mut s = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Supplier", cmd.id.to_string()))?;
        s.deactivate(cmd.updated_by);
        self.repo.save(&s).await?;
        Ok(s)
    }
}

register_command_handler!(DeactivateSupplier, AppDeps, |d: &AppDeps| {
    DeactivateSupplierHandler { repo: d.supplier_repo.clone() }
});

// ── DeleteSupplier ────────────────────────────────────────────────────────────

pub struct DeleteSupplierHandler {
    repo: Arc<dyn SupplierRepository>,
}

#[async_trait]
impl CommandHandler<DeleteSupplier> for DeleteSupplierHandler {
    async fn handle(&self, cmd: DeleteSupplier) -> AppResult<()> {
        self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Supplier", cmd.id.to_string()))?;
        self.repo.delete(cmd.id).await
    }
}

register_command_handler!(DeleteSupplier, AppDeps, |d: &AppDeps| {
    DeleteSupplierHandler { repo: d.supplier_repo.clone() }
});

// ── UpdateSupplierStatus ──────────────────────────────────────────────────────

pub struct UpdateSupplierStatusHandler {
    repo: Arc<dyn SupplierRepository>,
}

#[async_trait]
impl CommandHandler<UpdateSupplierStatus> for UpdateSupplierStatusHandler {
    async fn handle(&self, cmd: UpdateSupplierStatus) -> AppResult<Supplier> {
        let mut s = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Supplier", cmd.id.to_string()))?;
        s.update_status(cmd.status, cmd.updated_by);
        self.repo.save(&s).await?;
        Ok(s)
    }
}

register_command_handler!(UpdateSupplierStatus, AppDeps, |d: &AppDeps| {
    UpdateSupplierStatusHandler { repo: d.supplier_repo.clone() }
});

// ── UpdateOnboardingStatus ────────────────────────────────────────────────────

pub struct UpdateOnboardingStatusHandler {
    repo: Arc<dyn SupplierRepository>,
}

#[async_trait]
impl CommandHandler<UpdateOnboardingStatus> for UpdateOnboardingStatusHandler {
    async fn handle(&self, cmd: UpdateOnboardingStatus) -> AppResult<Supplier> {
        let mut s = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Supplier", cmd.id.to_string()))?;
        s.update_onboarding_status(cmd.onboarding_status, cmd.updated_by);
        self.repo.save(&s).await?;
        Ok(s)
    }
}

register_command_handler!(UpdateOnboardingStatus, AppDeps, |d: &AppDeps| {
    UpdateOnboardingStatusHandler { repo: d.supplier_repo.clone() }
});

// ── CreateSupplierContact ─────────────────────────────────────────────────────

pub struct CreateSupplierContactHandler {
    supplier_repo: Arc<dyn SupplierRepository>,
    contact_repo:  Arc<dyn SupplierContactRepository>,
}

#[async_trait]
impl CommandHandler<CreateSupplierContact> for CreateSupplierContactHandler {
    async fn handle(&self, cmd: CreateSupplierContact) -> AppResult<SupplierContact> {
        self.supplier_repo.find_by_id(cmd.supplier_id).await?
            .ok_or_else(|| AppError::not_found("Supplier", cmd.supplier_id.to_string()))?;
        let contact = SupplierContact::create(
            cmd.supplier_id, crate::domain::enums::ContactType::Primary,
            cmd.first_name, cmd.last_name, cmd.email, cmd.phone,
            cmd.position, cmd.is_primary, cmd.notes, cmd.created_by,
        );
        self.contact_repo.save(&contact).await?;
        Ok(contact)
    }
}

register_command_handler!(CreateSupplierContact, AppDeps, |d: &AppDeps| {
    CreateSupplierContactHandler {
        supplier_repo: d.supplier_repo.clone(),
        contact_repo:  d.contact_repo.clone(),
    }
});

// ── RequestDocumentUploadUrl ──────────────────────────────────────────────────

pub struct RequestDocumentUploadUrlHandler {
    supplier_repo:    Arc<dyn SupplierRepository>,
    blob_storage:     Arc<dyn BlobStorage>,
    blob_bucket:      String,
    presign_ttl_secs: u64,
}

#[async_trait]
impl CommandHandler<RequestDocumentUploadUrl> for RequestDocumentUploadUrlHandler {
    async fn handle(&self, cmd: RequestDocumentUploadUrl) -> AppResult<(String, String, String)> {
        self.supplier_repo.find_by_id(cmd.supplier_id).await?
            .ok_or_else(|| AppError::not_found("Supplier", cmd.supplier_id.to_string()))?;
        let ext = cmd.file_name.rsplit('.').next().unwrap_or("bin");
        let key = format!("suppliers/{}/{}.{}", cmd.supplier_id, uuid::Uuid::new_v4(), ext);
        let presigned = self.blob_storage
            .presigned_put(&self.blob_bucket, &key, &cmd.content_type, Duration::from_secs(self.presign_ttl_secs))
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        Ok((presigned.url, key, presigned.expires_at.to_rfc3339()))
    }
}

register_command_handler!(RequestDocumentUploadUrl, AppDeps, |d: &AppDeps| {
    RequestDocumentUploadUrlHandler {
        supplier_repo:    d.supplier_repo.clone(),
        blob_storage:     d.blob_storage.clone(),
        blob_bucket:      d.blob_bucket.clone(),
        presign_ttl_secs: d.presign_ttl_secs,
    }
});

// ── ConfirmDocumentUpload ─────────────────────────────────────────────────────

pub struct ConfirmDocumentUploadHandler {
    document_repo: Arc<dyn SupplierDocumentRepository>,
}

#[async_trait]
impl CommandHandler<ConfirmDocumentUpload> for ConfirmDocumentUploadHandler {
    async fn handle(&self, cmd: ConfirmDocumentUpload) -> AppResult<SupplierDocument> {
        let doc = SupplierDocument::create(
            cmd.supplier_id, cmd.file_name, cmd.object_name,
            cmd.content_type, cmd.document_type, cmd.created_by,
        );
        self.document_repo.save(&doc).await?;
        Ok(doc)
    }
}

register_command_handler!(ConfirmDocumentUpload, AppDeps, |d: &AppDeps| {
    ConfirmDocumentUploadHandler { document_repo: d.document_repo.clone() }
});

// ── DeleteSupplierDocument ────────────────────────────────────────────────────

pub struct DeleteSupplierDocumentHandler {
    document_repo: Arc<dyn SupplierDocumentRepository>,
}

#[async_trait]
impl CommandHandler<DeleteSupplierDocument> for DeleteSupplierDocumentHandler {
    async fn handle(&self, cmd: DeleteSupplierDocument) -> AppResult<()> {
        let doc = self.document_repo.find_by_id(cmd.document_id).await?
            .ok_or_else(|| AppError::not_found("Document", cmd.document_id.to_string()))?;
        if doc.supplier_id != cmd.supplier_id {
            return Err(AppError::not_found("Document", cmd.document_id.to_string()));
        }
        self.document_repo.delete(cmd.document_id).await
    }
}

register_command_handler!(DeleteSupplierDocument, AppDeps, |d: &AppDeps| {
    DeleteSupplierDocumentHandler { document_repo: d.document_repo.clone() }
});

// ── AddSupplierProduct ────────────────────────────────────────────────────────

pub struct AddSupplierProductHandler {
    supplier_repo: Arc<dyn SupplierRepository>,
    product_repo:  Arc<dyn SupplierProductRepository>,
}

#[async_trait]
impl CommandHandler<AddSupplierProduct> for AddSupplierProductHandler {
    async fn handle(&self, cmd: AddSupplierProduct) -> AppResult<SupplierProduct> {
        self.supplier_repo.find_by_id(cmd.supplier_id).await?
            .ok_or_else(|| AppError::not_found("Supplier", cmd.supplier_id.to_string()))?;
        if self.product_repo.exists(cmd.supplier_id, cmd.product_id, cmd.variant_id).await? {
            return Err(AppError::conflict("Supplier already has this product/variant"));
        }
        let product = SupplierProduct::create(
            cmd.supplier_id, cmd.product_id, cmd.variant_id, cmd.unit_cost,
            cmd.supplier_sku, cmd.lead_time_days, cmd.min_order_quantity,
            cmd.is_preferred, cmd.created_by,
        );
        self.product_repo.save(&product).await?;
        Ok(product)
    }
}

register_command_handler!(AddSupplierProduct, AppDeps, |d: &AppDeps| {
    AddSupplierProductHandler {
        supplier_repo: d.supplier_repo.clone(),
        product_repo:  d.product_repo.clone(),
    }
});

// ── RemoveSupplierProduct ─────────────────────────────────────────────────────

pub struct RemoveSupplierProductHandler {
    product_repo: Arc<dyn SupplierProductRepository>,
}

#[async_trait]
impl CommandHandler<RemoveSupplierProduct> for RemoveSupplierProductHandler {
    async fn handle(&self, cmd: RemoveSupplierProduct) -> AppResult<()> {
        let p = self.product_repo.find_by_id(cmd.supplier_product_id).await?
            .ok_or_else(|| AppError::not_found("SupplierProduct", cmd.supplier_product_id.to_string()))?;
        if p.supplier_id != cmd.supplier_id {
            return Err(AppError::not_found("SupplierProduct", cmd.supplier_product_id.to_string()));
        }
        self.product_repo.delete(cmd.supplier_product_id).await
    }
}

register_command_handler!(RemoveSupplierProduct, AppDeps, |d: &AppDeps| {
    RemoveSupplierProductHandler { product_repo: d.product_repo.clone() }
});

// ── Queries ───────────────────────────────────────────────────────────────────

pub struct GetSupplierHandler {
    repo: Arc<dyn SupplierRepository>,
}

#[async_trait]
impl QueryHandler<GetSupplier> for GetSupplierHandler {
    async fn handle(&self, q: GetSupplier) -> AppResult<Option<Supplier>> {
        self.repo.find_by_id(q.id).await
    }
}

register_query_handler!(GetSupplier, AppDeps, |d: &AppDeps| {
    GetSupplierHandler { repo: d.supplier_repo.clone() }
});

pub struct ListSuppliersHandler {
    repo: Arc<dyn SupplierRepository>,
}

#[async_trait]
impl QueryHandler<ListSuppliers> for ListSuppliersHandler {
    async fn handle(&self, q: ListSuppliers) -> AppResult<Page<Supplier>> {
        self.repo.get_paged(q.active_only, q.search.as_deref(), &q.req).await
    }
}

register_query_handler!(ListSuppliers, AppDeps, |d: &AppDeps| {
    ListSuppliersHandler { repo: d.supplier_repo.clone() }
});

pub struct GetSupplierAddressesHandler {
    repo: Arc<dyn SupplierAddressRepository>,
}

#[async_trait]
impl QueryHandler<GetSupplierAddresses> for GetSupplierAddressesHandler {
    async fn handle(&self, q: GetSupplierAddresses) -> AppResult<Vec<crate::domain::entities::SupplierAddress>> {
        self.repo.get_by_supplier(q.supplier_id).await
    }
}

register_query_handler!(GetSupplierAddresses, AppDeps, |d: &AppDeps| {
    GetSupplierAddressesHandler { repo: d.address_repo.clone() }
});

pub struct GetSupplierContactsHandler {
    repo: Arc<dyn SupplierContactRepository>,
}

#[async_trait]
impl QueryHandler<GetSupplierContacts> for GetSupplierContactsHandler {
    async fn handle(&self, q: GetSupplierContacts) -> AppResult<Vec<SupplierContact>> {
        self.repo.get_by_supplier(q.supplier_id).await
    }
}

register_query_handler!(GetSupplierContacts, AppDeps, |d: &AppDeps| {
    GetSupplierContactsHandler { repo: d.contact_repo.clone() }
});

pub struct GetSupplierDocumentsHandler {
    document_repo:    Arc<dyn SupplierDocumentRepository>,
    blob_storage:     Arc<dyn BlobStorage>,
    blob_bucket:      String,
    presign_ttl_secs: u64,
}

#[async_trait]
impl QueryHandler<GetSupplierDocuments> for GetSupplierDocumentsHandler {
    async fn handle(&self, q: GetSupplierDocuments) -> AppResult<Vec<(SupplierDocument, String, String)>> {
        let docs = self.document_repo.get_by_supplier(q.supplier_id).await?;
        let mut result = Vec::with_capacity(docs.len());
        for doc in docs {
            let (url, expires_at) = self.blob_storage
                .presigned_get(&self.blob_bucket, &doc.object_name, Duration::from_secs(self.presign_ttl_secs))
                .await
                .map(|p| (p.url, p.expires_at.to_rfc3339()))
                .unwrap_or_default();
            result.push((doc, url, expires_at));
        }
        Ok(result)
    }
}

register_query_handler!(GetSupplierDocuments, AppDeps, |d: &AppDeps| {
    GetSupplierDocumentsHandler {
        document_repo:    d.document_repo.clone(),
        blob_storage:     d.blob_storage.clone(),
        blob_bucket:      d.blob_bucket.clone(),
        presign_ttl_secs: d.presign_ttl_secs,
    }
});

pub struct ListSupplierProductsHandler {
    repo: Arc<dyn SupplierProductRepository>,
}

#[async_trait]
impl QueryHandler<ListSupplierProducts> for ListSupplierProductsHandler {
    async fn handle(&self, q: ListSupplierProducts) -> AppResult<Vec<SupplierProduct>> {
        self.repo.get_by_supplier(q.supplier_id).await
    }
}

register_query_handler!(ListSupplierProducts, AppDeps, |d: &AppDeps| {
    ListSupplierProductsHandler { repo: d.product_repo.clone() }
});
