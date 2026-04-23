use chrono::{DateTime, Utc};
use ddd_application::impl_command;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::entities::{PurchaseOrder, Supplier, SupplierContact, SupplierDocument, SupplierProduct};
use crate::domain::enums::{AddressType, ContactType, OnboardingStatus, SupplierStatus};
use crate::domain::ids::*;

// ── Supplier ──────────────────────────────────────────────────────────────────

pub struct CreateSupplier {
    pub company_name:              String,
    pub tax_identification_number: Option<String>,
    pub registration_number:       Option<String>,
    pub email:                     Option<String>,
    pub phone:                     Option<String>,
    pub website:                   Option<String>,
    pub business_type:             Option<String>,
    pub notes:                     Option<String>,
    pub created_by:                Option<String>,
    pub address_type:              AddressType,
    pub address_line1:             String,
    pub address_city:              String,
    pub address_postal:            String,
    pub address_country:           String,
    pub contact_type:              ContactType,
    pub contact_first_name:        String,
    pub contact_last_name:         String,
    pub contact_email:             Option<String>,
}
impl_command!(CreateSupplier, Supplier);

pub struct UpdateSupplier {
    pub id:                        SupplierId,
    pub company_name:              String,
    pub tax_identification_number: Option<String>,
    pub registration_number:       Option<String>,
    pub email:                     Option<String>,
    pub phone:                     Option<String>,
    pub website:                   Option<String>,
    pub business_type:             Option<String>,
    pub years_in_business:         Option<i32>,
    pub notes:                     Option<String>,
    pub updated_by:                Option<String>,
}
impl_command!(UpdateSupplier, Supplier);

pub struct ActivateSupplier {
    pub id:         SupplierId,
    pub updated_by: Option<String>,
}
impl_command!(ActivateSupplier, Supplier);

pub struct DeactivateSupplier {
    pub id:         SupplierId,
    pub updated_by: Option<String>,
}
impl_command!(DeactivateSupplier, Supplier);

pub struct DeleteSupplier {
    pub id: SupplierId,
}
impl_command!(DeleteSupplier, ());

pub struct UpdateSupplierStatus {
    pub id:         SupplierId,
    pub status:     SupplierStatus,
    pub updated_by: Option<String>,
}
impl_command!(UpdateSupplierStatus, Supplier);

pub struct UpdateOnboardingStatus {
    pub id:                SupplierId,
    pub onboarding_status: OnboardingStatus,
    pub updated_by:        Option<String>,
}
impl_command!(UpdateOnboardingStatus, Supplier);

// ── Contact ───────────────────────────────────────────────────────────────────

pub struct CreateSupplierContact {
    pub supplier_id: SupplierId,
    pub first_name:  String,
    pub last_name:   String,
    pub email:       Option<String>,
    pub phone:       Option<String>,
    pub position:    Option<String>,
    pub is_primary:  bool,
    pub notes:       Option<String>,
    pub created_by:  Option<String>,
}
impl_command!(CreateSupplierContact, SupplierContact);

// ── Documents ─────────────────────────────────────────────────────────────────

pub struct RequestDocumentUploadUrl {
    pub supplier_id:   SupplierId,
    pub file_name:     String,
    pub content_type:  String,
    pub document_type: Option<String>,
}
impl_command!(RequestDocumentUploadUrl, (String, String, String));

pub struct ConfirmDocumentUpload {
    pub supplier_id:   SupplierId,
    pub object_name:   String,
    pub file_name:     String,
    pub content_type:  String,
    pub document_type: Option<String>,
    pub created_by:    Option<String>,
}
impl_command!(ConfirmDocumentUpload, SupplierDocument);

pub struct DeleteSupplierDocument {
    pub supplier_id: SupplierId,
    pub document_id: DocumentId,
}
impl_command!(DeleteSupplierDocument, ());

// ── Supplier Products ─────────────────────────────────────────────────────────

pub struct AddSupplierProduct {
    pub supplier_id:        SupplierId,
    pub product_id:         Uuid,
    pub variant_id:         Option<Uuid>,
    pub unit_cost:          Decimal,
    pub supplier_sku:       Option<String>,
    pub lead_time_days:     Option<i32>,
    pub min_order_quantity: Option<i32>,
    pub is_preferred:       bool,
    pub created_by:         Option<String>,
}
impl_command!(AddSupplierProduct, SupplierProduct);

pub struct RemoveSupplierProduct {
    pub supplier_id:         SupplierId,
    pub supplier_product_id: SupplierProductId,
}
impl_command!(RemoveSupplierProduct, ());

// ── Purchase Orders ───────────────────────────────────────────────────────────

pub struct PurchaseOrderDetailInput {
    pub product_id: Uuid,
    pub quantity:   i32,
    pub unit_cost:  Decimal,
}

pub struct CreatePurchaseOrder {
    pub supplier_id:         SupplierId,
    pub store_id:            i32,
    pub expected_date:       Option<DateTime<Utc>>,
    pub shipping_address_id: Option<AddressId>,
    pub contact_person_id:   Option<ContactId>,
    pub created_by:          Option<Uuid>,
    pub order_details:       Vec<PurchaseOrderDetailInput>,
}
impl_command!(CreatePurchaseOrder, PurchaseOrder);

pub struct SubmitPurchaseOrder {
    pub id:         OrderId,
    pub updated_by: Option<Uuid>,
}
impl_command!(SubmitPurchaseOrder, PurchaseOrder);

pub struct CancelPurchaseOrder {
    pub id:         OrderId,
    pub updated_by: Option<Uuid>,
}
impl_command!(CancelPurchaseOrder, PurchaseOrder);
