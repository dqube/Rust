use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use super::enums::{AddressType, ContactType, OnboardingStatus, PurchaseOrderStatus, SupplierStatus};
use super::ids::*;

// ── Supplier ──────────────────────────────────────────────────────────────────

pub struct Supplier {
    pub id:                        SupplierId,
    pub user_id:                   Option<Uuid>,
    pub supplier_code:             String,
    pub company_name:              String,
    pub tax_identification_number: Option<String>,
    pub registration_number:       Option<String>,
    pub email:                     Option<String>,
    pub phone:                     Option<String>,
    pub website:                   Option<String>,
    pub business_type:             Option<String>,
    pub years_in_business:         Option<i32>,
    pub status:                    SupplierStatus,
    pub onboarding_status:         OnboardingStatus,
    pub rating:                    Option<Decimal>,
    pub total_orders:              i32,
    pub notes:                     Option<String>,
    pub created_at:                DateTime<Utc>,
    pub created_by:                Option<String>,
    pub updated_at:                Option<DateTime<Utc>>,
    pub updated_by:                Option<String>,
}

impl Supplier {
    fn generate_code(company_name: &str) -> String {
        let prefix: String = company_name.chars().filter(|c| c.is_alphabetic()).take(3)
            .collect::<String>().to_uppercase();
        let prefix = if prefix.len() < 3 { format!("{:S<3}", prefix) } else { prefix };
        format!("{}-{}", prefix, Utc::now().timestamp_millis())
    }

    pub fn create(
        company_name: String,
        tax_identification_number: Option<String>,
        registration_number: Option<String>,
        email: Option<String>,
        phone: Option<String>,
        website: Option<String>,
        business_type: Option<String>,
        notes: Option<String>,
        created_by: Option<String>,
    ) -> Self {
        let code = Self::generate_code(&company_name);
        Self {
            id: SupplierId::new(),
            user_id: None,
            supplier_code: code,
            company_name,
            tax_identification_number,
            registration_number,
            email,
            phone,
            website,
            business_type,
            years_in_business: None,
            status: SupplierStatus::Pending,
            onboarding_status: OnboardingStatus::Pending,
            rating: None,
            total_orders: 0,
            notes,
            created_at: Utc::now(),
            created_by,
            updated_at: None,
            updated_by: None,
        }
    }

    pub fn update(
        &mut self,
        company_name: String,
        tax_identification_number: Option<String>,
        registration_number: Option<String>,
        email: Option<String>,
        phone: Option<String>,
        website: Option<String>,
        business_type: Option<String>,
        years_in_business: Option<i32>,
        notes: Option<String>,
        updated_by: Option<String>,
    ) {
        self.company_name              = company_name;
        self.tax_identification_number = tax_identification_number;
        self.registration_number       = registration_number;
        self.email                     = email;
        self.phone                     = phone;
        self.website                   = website;
        self.business_type             = business_type;
        self.years_in_business         = years_in_business;
        self.notes                     = notes;
        self.updated_at                = Some(Utc::now());
        self.updated_by                = updated_by;
    }

    pub fn activate(&mut self, updated_by: Option<String>) {
        self.status     = SupplierStatus::Active;
        self.updated_at = Some(Utc::now());
        self.updated_by = updated_by;
    }

    pub fn deactivate(&mut self, updated_by: Option<String>) {
        self.status     = SupplierStatus::Inactive;
        self.updated_at = Some(Utc::now());
        self.updated_by = updated_by;
    }

    pub fn update_status(&mut self, status: SupplierStatus, updated_by: Option<String>) {
        self.status     = status;
        self.updated_at = Some(Utc::now());
        self.updated_by = updated_by;
    }

    pub fn update_onboarding_status(&mut self, onboarding_status: OnboardingStatus, updated_by: Option<String>) {
        self.onboarding_status = onboarding_status;
        self.updated_at        = Some(Utc::now());
        self.updated_by        = updated_by;
    }
}

// ── SupplierAddress ───────────────────────────────────────────────────────────

pub struct SupplierAddress {
    pub id:           AddressId,
    pub supplier_id:  SupplierId,
    pub address_type: AddressType,
    pub line1:        String,
    pub line2:        Option<String>,
    pub city:         String,
    pub state:        Option<String>,
    pub postal_code:  String,
    pub country:      String,
    pub is_primary:   bool,
    pub notes:        Option<String>,
    pub created_at:   DateTime<Utc>,
    pub created_by:   Option<Uuid>,
    pub updated_at:   Option<DateTime<Utc>>,
    pub updated_by:   Option<Uuid>,
}

impl SupplierAddress {
    pub fn create(
        supplier_id: SupplierId,
        address_type: AddressType,
        line1: String,
        city: String,
        postal_code: String,
        country: String,
        line2: Option<String>,
        state: Option<String>,
        is_primary: bool,
        notes: Option<String>,
        created_by: Option<Uuid>,
    ) -> Self {
        Self {
            id: AddressId::new(),
            supplier_id,
            address_type,
            line1,
            line2,
            city,
            state,
            postal_code,
            country,
            is_primary,
            notes,
            created_at: Utc::now(),
            created_by,
            updated_at: None,
            updated_by: None,
        }
    }
}

// ── SupplierContact ───────────────────────────────────────────────────────────

pub struct SupplierContact {
    pub id:           ContactId,
    pub supplier_id:  SupplierId,
    pub user_id:      Option<Uuid>,
    pub contact_type: ContactType,
    pub first_name:   String,
    pub last_name:    String,
    pub email:        Option<String>,
    pub phone:        Option<String>,
    pub mobile:       Option<String>,
    pub position:     Option<String>,
    pub department:   Option<String>,
    pub is_primary:   bool,
    pub can_login:    bool,
    pub notes:        Option<String>,
    pub created_at:   DateTime<Utc>,
    pub created_by:   Option<String>,
    pub updated_at:   Option<DateTime<Utc>>,
    pub updated_by:   Option<String>,
}

impl SupplierContact {
    pub fn create(
        supplier_id: SupplierId,
        contact_type: ContactType,
        first_name: String,
        last_name: String,
        email: Option<String>,
        phone: Option<String>,
        position: Option<String>,
        is_primary: bool,
        notes: Option<String>,
        created_by: Option<String>,
    ) -> Self {
        Self {
            id: ContactId::new(),
            supplier_id,
            user_id: None,
            contact_type,
            first_name,
            last_name,
            email,
            phone,
            mobile: None,
            position,
            department: None,
            is_primary,
            can_login: false,
            notes,
            created_at: Utc::now(),
            created_by,
            updated_at: None,
            updated_by: None,
        }
    }
}

// ── SupplierDocument ──────────────────────────────────────────────────────────

pub struct SupplierDocument {
    pub id:            DocumentId,
    pub supplier_id:   SupplierId,
    pub file_name:     String,
    pub object_name:   String,
    pub content_type:  String,
    pub document_type: Option<String>,
    pub created_at:    DateTime<Utc>,
    pub created_by:    Option<String>,
}

impl SupplierDocument {
    pub fn create(
        supplier_id: SupplierId,
        file_name: String,
        object_name: String,
        content_type: String,
        document_type: Option<String>,
        created_by: Option<String>,
    ) -> Self {
        Self {
            id: DocumentId::new(),
            supplier_id,
            file_name,
            object_name,
            content_type,
            document_type,
            created_at: Utc::now(),
            created_by,
        }
    }
}

// ── SupplierProduct ───────────────────────────────────────────────────────────

pub struct SupplierProduct {
    pub id:                 SupplierProductId,
    pub supplier_id:        SupplierId,
    pub product_id:         Uuid,
    pub variant_id:         Option<Uuid>,
    pub supplier_sku:       Option<String>,
    pub unit_cost:          Decimal,
    pub lead_time_days:     Option<i32>,
    pub min_order_quantity: Option<i32>,
    pub is_preferred:       bool,
    pub created_at:         DateTime<Utc>,
    pub created_by:         Option<String>,
    pub updated_at:         Option<DateTime<Utc>>,
    pub updated_by:         Option<String>,
}

impl SupplierProduct {
    pub fn create(
        supplier_id: SupplierId,
        product_id: Uuid,
        variant_id: Option<Uuid>,
        unit_cost: Decimal,
        supplier_sku: Option<String>,
        lead_time_days: Option<i32>,
        min_order_quantity: Option<i32>,
        is_preferred: bool,
        created_by: Option<String>,
    ) -> Self {
        Self {
            id: SupplierProductId::new(),
            supplier_id,
            product_id,
            variant_id,
            supplier_sku,
            unit_cost,
            lead_time_days,
            min_order_quantity,
            is_preferred,
            created_at: Utc::now(),
            created_by,
            updated_at: None,
            updated_by: None,
        }
    }
}

// ── PurchaseOrderDetail ───────────────────────────────────────────────────────

pub struct PurchaseOrderDetail {
    pub id:                OrderDetailId,
    pub order_id:          OrderId,
    pub product_id:        Uuid,
    pub quantity:          i32,
    pub unit_cost:         Decimal,
    pub received_quantity: Option<i32>,
    pub created_at:        DateTime<Utc>,
    pub created_by:        Option<Uuid>,
}

// ── PurchaseOrder ─────────────────────────────────────────────────────────────

pub struct PurchaseOrder {
    pub id:                  OrderId,
    pub supplier_id:         SupplierId,
    pub store_id:            i32,
    pub order_date:          DateTime<Utc>,
    pub expected_date:       Option<DateTime<Utc>>,
    pub status:              PurchaseOrderStatus,
    pub total_amount:        Decimal,
    pub shipping_address_id: Option<AddressId>,
    pub contact_person_id:   Option<ContactId>,
    pub created_at:          DateTime<Utc>,
    pub created_by:          Option<Uuid>,
    pub updated_at:          Option<DateTime<Utc>>,
    pub updated_by:          Option<Uuid>,
    pub order_details:       Vec<PurchaseOrderDetail>,
}

impl PurchaseOrder {
    pub fn create(
        supplier_id: SupplierId,
        store_id: i32,
        expected_date: Option<DateTime<Utc>>,
        shipping_address_id: Option<AddressId>,
        contact_person_id: Option<ContactId>,
        created_by: Option<Uuid>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: OrderId::new(),
            supplier_id,
            store_id,
            order_date: now,
            expected_date,
            status: PurchaseOrderStatus::Draft,
            total_amount: Decimal::ZERO,
            shipping_address_id,
            contact_person_id,
            created_at: now,
            created_by,
            updated_at: None,
            updated_by: None,
            order_details: Vec::new(),
        }
    }

    pub fn add_detail(&mut self, product_id: Uuid, quantity: i32, unit_cost: Decimal, created_by: Option<Uuid>) {
        let order_id = self.id;
        self.order_details.push(PurchaseOrderDetail {
            id: OrderDetailId::new(),
            order_id,
            product_id,
            quantity,
            unit_cost,
            received_quantity: None,
            created_at: Utc::now(),
            created_by,
        });
        self.recalculate_total();
    }

    fn recalculate_total(&mut self) {
        self.total_amount = self.order_details.iter()
            .map(|d| d.unit_cost * Decimal::from(d.quantity))
            .sum();
    }

    pub fn submit(&mut self, updated_by: Option<Uuid>) -> Result<(), String> {
        if self.status != PurchaseOrderStatus::Draft {
            return Err(format!("Cannot submit order in status {:?}", self.status));
        }
        self.status     = PurchaseOrderStatus::Ordered;
        self.updated_at = Some(Utc::now());
        self.updated_by = updated_by;
        Ok(())
    }

    pub fn cancel(&mut self, updated_by: Option<Uuid>) -> Result<(), String> {
        if self.status == PurchaseOrderStatus::Cancelled {
            return Err("Order already cancelled".into());
        }
        self.status     = PurchaseOrderStatus::Cancelled;
        self.updated_at = Some(Utc::now());
        self.updated_by = updated_by;
        Ok(())
    }
}
