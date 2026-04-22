use chrono::{DateTime, Utc};
use ddd_application::impl_command;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::entities::{PurchaseOrder, Supplier, SupplierContact, SupplierDocument, SupplierProduct};
use crate::domain::enums::{AddressType, ContactType, OnboardingStatus, SupplierStatus};
use crate::domain::ids::*;

// ── Supplier ──────────────────────────────────────────────────────────────────

impl_command! {
    CreateSupplier {
        company_name:              String,
        tax_identification_number: Option<String>,
        registration_number:       Option<String>,
        email:                     Option<String>,
        phone:                     Option<String>,
        website:                   Option<String>,
        business_type:             Option<String>,
        notes:                     Option<String>,
        created_by:                Option<String>,
        // Initial address
        address_type:   AddressType,
        address_line1:  String,
        address_city:   String,
        address_postal: String,
        address_country: String,
        // Initial contact
        contact_type:       ContactType,
        contact_first_name: String,
        contact_last_name:  String,
        contact_email:      Option<String>,
    } -> Supplier
}

impl_command! {
    UpdateSupplier {
        id:                        SupplierId,
        company_name:              String,
        tax_identification_number: Option<String>,
        registration_number:       Option<String>,
        email:                     Option<String>,
        phone:                     Option<String>,
        website:                   Option<String>,
        business_type:             Option<String>,
        years_in_business:         Option<i32>,
        notes:                     Option<String>,
        updated_by:                Option<String>,
    } -> Supplier
}

impl_command! {
    ActivateSupplier   { id: SupplierId, updated_by: Option<String> } -> Supplier
}
impl_command! {
    DeactivateSupplier { id: SupplierId, updated_by: Option<String> } -> Supplier
}
impl_command! {
    DeleteSupplier     { id: SupplierId } -> ()
}

impl_command! {
    UpdateSupplierStatus {
        id:         SupplierId,
        status:     SupplierStatus,
        updated_by: Option<String>,
    } -> Supplier
}

impl_command! {
    UpdateOnboardingStatus {
        id:                SupplierId,
        onboarding_status: OnboardingStatus,
        updated_by:        Option<String>,
    } -> Supplier
}

// ── Contact ───────────────────────────────────────────────────────────────────

impl_command! {
    CreateSupplierContact {
        supplier_id: SupplierId,
        first_name:  String,
        last_name:   String,
        email:       Option<String>,
        phone:       Option<String>,
        position:    Option<String>,
        is_primary:  bool,
        notes:       Option<String>,
        created_by:  Option<String>,
    } -> SupplierContact
}

// ── Documents (presigned) ─────────────────────────────────────────────────────

impl_command! {
    RequestDocumentUploadUrl {
        supplier_id:   SupplierId,
        file_name:     String,
        content_type:  String,
        document_type: Option<String>,
    } -> (String, String, String)  // (upload_url, object_name, expires_at)
}

impl_command! {
    ConfirmDocumentUpload {
        supplier_id:   SupplierId,
        object_name:   String,
        file_name:     String,
        content_type:  String,
        document_type: Option<String>,
        created_by:    Option<String>,
    } -> SupplierDocument
}

impl_command! {
    DeleteSupplierDocument {
        supplier_id: SupplierId,
        document_id: DocumentId,
    } -> ()
}

// ── Supplier Products ─────────────────────────────────────────────────────────

impl_command! {
    AddSupplierProduct {
        supplier_id:        SupplierId,
        product_id:         Uuid,
        variant_id:         Option<Uuid>,
        unit_cost:          Decimal,
        supplier_sku:       Option<String>,
        lead_time_days:     Option<i32>,
        min_order_quantity: Option<i32>,
        is_preferred:       bool,
        created_by:         Option<String>,
    } -> SupplierProduct
}

impl_command! {
    RemoveSupplierProduct {
        supplier_id:        SupplierId,
        supplier_product_id: SupplierProductId,
    } -> ()
}

// ── Purchase Orders ───────────────────────────────────────────────────────────

pub struct PurchaseOrderDetailInput {
    pub product_id: Uuid,
    pub quantity:   i32,
    pub unit_cost:  Decimal,
}

impl_command! {
    CreatePurchaseOrder {
        supplier_id:         SupplierId,
        store_id:            i32,
        expected_date:       Option<DateTime<Utc>>,
        shipping_address_id: Option<AddressId>,
        contact_person_id:   Option<ContactId>,
        created_by:          Option<Uuid>,
        order_details:       Vec<PurchaseOrderDetailInput>,
    } -> PurchaseOrder
}

impl_command! {
    SubmitPurchaseOrder  { id: OrderId, updated_by: Option<Uuid> } -> PurchaseOrder
}
impl_command! {
    CancelPurchaseOrder  { id: OrderId, updated_by: Option<Uuid> } -> PurchaseOrder
}
