use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use ddd_shared_kernel::{AppError, Page, PageRequest};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};
use uuid::Uuid;

use crate::domain::entities::*;
use crate::domain::enums::*;
use crate::domain::ids::*;
use crate::domain::repositories::*;
use crate::infrastructure::db::models::*;

// ── helpers ───────────────────────────────────────────────────────────────────

fn db_err(e: sea_orm::DbErr) -> AppError {
    AppError::internal(e.to_string())
}

fn to_utc(dt: sea_orm::prelude::DateTimeWithTimeZone) -> DateTime<Utc> {
    dt.with_timezone(&Utc)
}
fn opt_to_utc(dt: Option<sea_orm::prelude::DateTimeWithTimeZone>) -> Option<DateTime<Utc>> {
    dt.map(to_utc)
}
fn from_utc(dt: DateTime<Utc>) -> sea_orm::prelude::DateTimeWithTimeZone {
    dt.fixed_offset()
}
fn opt_from_utc(dt: Option<DateTime<Utc>>) -> Option<sea_orm::prelude::DateTimeWithTimeZone> {
    dt.map(|d| d.fixed_offset())
}

// ── model → domain mappers ────────────────────────────────────────────────────

fn m2supplier(m: supplier::Model) -> Supplier {
    Supplier {
        id:                        SupplierId::from_uuid(m.id),
        user_id:                   m.user_id,
        supplier_code:             m.supplier_code,
        company_name:              m.company_name,
        tax_identification_number: m.tax_identification_number,
        registration_number:       m.registration_number,
        email:                     m.email,
        phone:                     m.phone,
        website:                   m.website,
        business_type:             m.business_type,
        years_in_business:         m.years_in_business,
        status:                    SupplierStatus::from_i32(m.status),
        onboarding_status:         OnboardingStatus::from_i32(m.onboarding_status),
        rating:                    m.rating,
        total_orders:              m.total_orders,
        notes:                     m.notes,
        created_at:                to_utc(m.created_at),
        created_by:                m.created_by,
        updated_at:                opt_to_utc(m.updated_at),
        updated_by:                m.updated_by,
    }
}

fn m2address(m: supplier_address::Model) -> SupplierAddress {
    SupplierAddress {
        id:           AddressId::from_uuid(m.id),
        supplier_id:  SupplierId::from_uuid(m.supplier_id),
        address_type: AddressType::from_i32(m.address_type),
        line1:        m.line1,
        line2:        m.line2,
        city:         m.city,
        state:        m.state,
        postal_code:  m.postal_code,
        country:      m.country,
        is_primary:   m.is_primary,
        notes:        m.notes,
        created_at:   to_utc(m.created_at),
        created_by:   m.created_by,
        updated_at:   opt_to_utc(m.updated_at),
        updated_by:   m.updated_by,
    }
}

fn m2contact(m: supplier_contact::Model) -> SupplierContact {
    SupplierContact {
        id:           ContactId::from_uuid(m.id),
        supplier_id:  SupplierId::from_uuid(m.supplier_id),
        user_id:      m.user_id,
        contact_type: ContactType::from_i32(m.contact_type),
        first_name:   m.first_name,
        last_name:    m.last_name,
        email:        m.email,
        phone:        m.phone,
        mobile:       m.mobile,
        position:     m.position,
        department:   m.department,
        is_primary:   m.is_primary,
        can_login:    m.can_login,
        notes:        m.notes,
        created_at:   to_utc(m.created_at),
        created_by:   m.created_by,
        updated_at:   opt_to_utc(m.updated_at),
        updated_by:   m.updated_by,
    }
}

fn m2document(m: supplier_document::Model) -> SupplierDocument {
    SupplierDocument {
        id:            DocumentId::from_uuid(m.id),
        supplier_id:   SupplierId::from_uuid(m.supplier_id),
        file_name:     m.file_name,
        object_name:   m.object_name,
        content_type:  m.content_type,
        document_type: m.document_type,
        created_at:    to_utc(m.created_at),
        created_by:    m.created_by,
    }
}

fn m2product(m: supplier_product::Model) -> SupplierProduct {
    SupplierProduct {
        id:                 SupplierProductId::from_uuid(m.id),
        supplier_id:        SupplierId::from_uuid(m.supplier_id),
        product_id:         m.product_id,
        variant_id:         m.variant_id,
        supplier_sku:       m.supplier_sku,
        unit_cost:          m.unit_cost,
        lead_time_days:     m.lead_time_days,
        min_order_quantity: m.min_order_quantity,
        is_preferred:       m.is_preferred,
        created_at:         to_utc(m.created_at),
        created_by:         m.created_by,
        updated_at:         opt_to_utc(m.updated_at),
        updated_by:         m.updated_by,
    }
}

fn m2order_detail(m: purchase_order_detail::Model) -> PurchaseOrderDetail {
    PurchaseOrderDetail {
        id:                OrderDetailId::from_uuid(m.id),
        order_id:          OrderId::from_uuid(m.order_id),
        product_id:        m.product_id,
        quantity:          m.quantity,
        unit_cost:         m.unit_cost,
        received_quantity: m.received_quantity,
        created_at:        to_utc(m.created_at),
        created_by:        m.created_by,
    }
}

fn m2order(m: purchase_order::Model, details: Vec<PurchaseOrderDetail>) -> PurchaseOrder {
    PurchaseOrder {
        id:                  OrderId::from_uuid(m.id),
        supplier_id:         SupplierId::from_uuid(m.supplier_id),
        store_id:            m.store_id,
        order_date:          to_utc(m.order_date),
        expected_date:       opt_to_utc(m.expected_date),
        status:              PurchaseOrderStatus::from_str(&m.status),
        total_amount:        m.total_amount,
        shipping_address_id: m.shipping_address_id.map(AddressId::from_uuid),
        contact_person_id:   m.contact_person_id.map(ContactId::from_uuid),
        created_at:          to_utc(m.created_at),
        created_by:          m.created_by,
        updated_at:          opt_to_utc(m.updated_at),
        updated_by:          m.updated_by,
        order_details:       details,
    }
}

// ── PgSupplierRepository ──────────────────────────────────────────────────────

pub struct PgSupplierRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl SupplierRepository for PgSupplierRepository {
    async fn find_by_id(&self, id: SupplierId) -> Result<Option<Supplier>, AppError> {
        Ok(supplier::Entity::find_by_id(id.as_uuid()).one(&*self.0).await.map_err(db_err)?.map(m2supplier))
    }

    async fn get_paged(&self, active_only: bool, search: Option<&str>, req: &PageRequest) -> Result<Page<Supplier>, AppError> {
        let mut q = supplier::Entity::find();
        if active_only {
            q = q.filter(supplier::Column::Status.eq(1i32));
        }
        if let Some(s) = search {
            if !s.is_empty() {
                q = q.filter(supplier::Column::CompanyName.contains(s.to_lowercase()));
            }
        }
        let total    = q.clone().count(&*self.0).await.map_err(db_err)? as u64;
        let page     = req.page().max(1);
        let per_page = req.per_page().max(1);
        let offset   = ((page - 1) * per_page) as u64;
        let items    = q.order_by_asc(supplier::Column::CompanyName)
            .offset(offset).limit(per_page as u64)
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(m2supplier).collect();
        Ok(Page::new(items, total, page, per_page))
    }

    async fn code_exists(&self, code: &str) -> Result<bool, AppError> {
        Ok(supplier::Entity::find()
            .filter(supplier::Column::SupplierCode.eq(code.to_string()))
            .count(&*self.0).await.map_err(db_err)? > 0)
    }

    async fn email_exists(&self, email: &str) -> Result<bool, AppError> {
        Ok(supplier::Entity::find()
            .filter(supplier::Column::Email.eq(email.to_string()))
            .count(&*self.0).await.map_err(db_err)? > 0)
    }

    async fn save(&self, s: &Supplier) -> Result<(), AppError> {
        let active = supplier::ActiveModel {
            id:                        Set(s.id.as_uuid()),
            user_id:                   Set(s.user_id),
            supplier_code:             Set(s.supplier_code.clone()),
            company_name:              Set(s.company_name.clone()),
            tax_identification_number: Set(s.tax_identification_number.clone()),
            registration_number:       Set(s.registration_number.clone()),
            email:                     Set(s.email.clone()),
            phone:                     Set(s.phone.clone()),
            website:                   Set(s.website.clone()),
            business_type:             Set(s.business_type.clone()),
            years_in_business:         Set(s.years_in_business),
            status:                    Set(s.status.as_i32()),
            onboarding_status:         Set(s.onboarding_status.as_i32()),
            rating:                    Set(s.rating),
            total_orders:              Set(s.total_orders),
            notes:                     Set(s.notes.clone()),
            created_at:                Set(from_utc(s.created_at)),
            created_by:                Set(s.created_by.clone()),
            updated_at:                Set(opt_from_utc(s.updated_at)),
            updated_by:                Set(s.updated_by.clone()),
        };
        supplier::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(supplier::Column::Id)
                    .update_columns([
                        supplier::Column::UserId,
                        supplier::Column::CompanyName,
                        supplier::Column::TaxIdentificationNumber,
                        supplier::Column::RegistrationNumber,
                        supplier::Column::Email,
                        supplier::Column::Phone,
                        supplier::Column::Website,
                        supplier::Column::BusinessType,
                        supplier::Column::YearsInBusiness,
                        supplier::Column::Status,
                        supplier::Column::OnboardingStatus,
                        supplier::Column::Rating,
                        supplier::Column::TotalOrders,
                        supplier::Column::Notes,
                        supplier::Column::UpdatedAt,
                        supplier::Column::UpdatedBy,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }

    async fn delete(&self, id: SupplierId) -> Result<(), AppError> {
        supplier::Entity::delete_by_id(id.as_uuid()).exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
}

// ── PgSupplierAddressRepository ───────────────────────────────────────────────

pub struct PgSupplierAddressRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl SupplierAddressRepository for PgSupplierAddressRepository {
    async fn find_by_id(&self, id: AddressId) -> Result<Option<SupplierAddress>, AppError> {
        Ok(supplier_address::Entity::find_by_id(id.as_uuid()).one(&*self.0).await.map_err(db_err)?.map(m2address))
    }
    async fn get_by_supplier(&self, supplier_id: SupplierId) -> Result<Vec<SupplierAddress>, AppError> {
        Ok(supplier_address::Entity::find()
            .filter(supplier_address::Column::SupplierId.eq(supplier_id.as_uuid()))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(m2address).collect())
    }
    async fn save(&self, a: &SupplierAddress) -> Result<(), AppError> {
        let active = supplier_address::ActiveModel {
            id:           Set(a.id.as_uuid()),
            supplier_id:  Set(a.supplier_id.as_uuid()),
            address_type: Set(a.address_type.as_i32()),
            line1:        Set(a.line1.clone()),
            line2:        Set(a.line2.clone()),
            city:         Set(a.city.clone()),
            state:        Set(a.state.clone()),
            postal_code:  Set(a.postal_code.clone()),
            country:      Set(a.country.clone()),
            is_primary:   Set(a.is_primary),
            notes:        Set(a.notes.clone()),
            created_at:   Set(from_utc(a.created_at)),
            created_by:   Set(a.created_by),
            updated_at:   Set(opt_from_utc(a.updated_at)),
            updated_by:   Set(a.updated_by),
        };
        supplier_address::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(supplier_address::Column::Id)
                    .update_columns([
                        supplier_address::Column::AddressType,
                        supplier_address::Column::Line1, supplier_address::Column::Line2,
                        supplier_address::Column::City,  supplier_address::Column::State,
                        supplier_address::Column::PostalCode, supplier_address::Column::Country,
                        supplier_address::Column::IsPrimary,  supplier_address::Column::Notes,
                        supplier_address::Column::UpdatedAt,  supplier_address::Column::UpdatedBy,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
}

// ── PgSupplierContactRepository ───────────────────────────────────────────────

pub struct PgSupplierContactRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl SupplierContactRepository for PgSupplierContactRepository {
    async fn find_by_id(&self, id: ContactId) -> Result<Option<SupplierContact>, AppError> {
        Ok(supplier_contact::Entity::find_by_id(id.as_uuid()).one(&*self.0).await.map_err(db_err)?.map(m2contact))
    }
    async fn get_by_supplier(&self, supplier_id: SupplierId) -> Result<Vec<SupplierContact>, AppError> {
        Ok(supplier_contact::Entity::find()
            .filter(supplier_contact::Column::SupplierId.eq(supplier_id.as_uuid()))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(m2contact).collect())
    }
    async fn save(&self, c: &SupplierContact) -> Result<(), AppError> {
        let active = supplier_contact::ActiveModel {
            id:           Set(c.id.as_uuid()),
            supplier_id:  Set(c.supplier_id.as_uuid()),
            user_id:      Set(c.user_id),
            contact_type: Set(c.contact_type.as_i32()),
            first_name:   Set(c.first_name.clone()),
            last_name:    Set(c.last_name.clone()),
            email:        Set(c.email.clone()),
            phone:        Set(c.phone.clone()),
            mobile:       Set(c.mobile.clone()),
            position:     Set(c.position.clone()),
            department:   Set(c.department.clone()),
            is_primary:   Set(c.is_primary),
            can_login:    Set(c.can_login),
            notes:        Set(c.notes.clone()),
            created_at:   Set(from_utc(c.created_at)),
            created_by:   Set(c.created_by.clone()),
            updated_at:   Set(opt_from_utc(c.updated_at)),
            updated_by:   Set(c.updated_by.clone()),
        };
        supplier_contact::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(supplier_contact::Column::Id)
                    .update_columns([
                        supplier_contact::Column::UserId,
                        supplier_contact::Column::ContactType,
                        supplier_contact::Column::FirstName, supplier_contact::Column::LastName,
                        supplier_contact::Column::Email,     supplier_contact::Column::Phone,
                        supplier_contact::Column::Mobile,    supplier_contact::Column::Position,
                        supplier_contact::Column::Department, supplier_contact::Column::IsPrimary,
                        supplier_contact::Column::CanLogin,   supplier_contact::Column::Notes,
                        supplier_contact::Column::UpdatedAt,  supplier_contact::Column::UpdatedBy,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
}

// ── PgSupplierDocumentRepository ──────────────────────────────────────────────

pub struct PgSupplierDocumentRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl SupplierDocumentRepository for PgSupplierDocumentRepository {
    async fn find_by_id(&self, id: DocumentId) -> Result<Option<SupplierDocument>, AppError> {
        Ok(supplier_document::Entity::find_by_id(id.as_uuid()).one(&*self.0).await.map_err(db_err)?.map(m2document))
    }
    async fn get_by_supplier(&self, supplier_id: SupplierId) -> Result<Vec<SupplierDocument>, AppError> {
        Ok(supplier_document::Entity::find()
            .filter(supplier_document::Column::SupplierId.eq(supplier_id.as_uuid()))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(m2document).collect())
    }
    async fn save(&self, d: &SupplierDocument) -> Result<(), AppError> {
        let active = supplier_document::ActiveModel {
            id:            Set(d.id.as_uuid()),
            supplier_id:   Set(d.supplier_id.as_uuid()),
            file_name:     Set(d.file_name.clone()),
            object_name:   Set(d.object_name.clone()),
            content_type:  Set(d.content_type.clone()),
            document_type: Set(d.document_type.clone()),
            created_at:    Set(from_utc(d.created_at)),
            created_by:    Set(d.created_by.clone()),
        };
        supplier_document::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(supplier_document::Column::Id)
                    .update_columns([supplier_document::Column::DocumentType])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
    async fn delete(&self, id: DocumentId) -> Result<(), AppError> {
        supplier_document::Entity::delete_by_id(id.as_uuid()).exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
}

// ── PgSupplierProductRepository ───────────────────────────────────────────────

pub struct PgSupplierProductRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl SupplierProductRepository for PgSupplierProductRepository {
    async fn find_by_id(&self, id: SupplierProductId) -> Result<Option<SupplierProduct>, AppError> {
        Ok(supplier_product::Entity::find_by_id(id.as_uuid()).one(&*self.0).await.map_err(db_err)?.map(m2product))
    }
    async fn get_by_supplier(&self, supplier_id: SupplierId) -> Result<Vec<SupplierProduct>, AppError> {
        Ok(supplier_product::Entity::find()
            .filter(supplier_product::Column::SupplierId.eq(supplier_id.as_uuid()))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(m2product).collect())
    }
    async fn exists(&self, supplier_id: SupplierId, product_id: Uuid, variant_id: Option<Uuid>) -> Result<bool, AppError> {
        let mut q = supplier_product::Entity::find()
            .filter(supplier_product::Column::SupplierId.eq(supplier_id.as_uuid()))
            .filter(supplier_product::Column::ProductId.eq(product_id));
        if let Some(vid) = variant_id {
            q = q.filter(supplier_product::Column::VariantId.eq(vid));
        } else {
            q = q.filter(supplier_product::Column::VariantId.is_null());
        }
        Ok(q.count(&*self.0).await.map_err(db_err)? > 0)
    }
    async fn save(&self, p: &SupplierProduct) -> Result<(), AppError> {
        let active = supplier_product::ActiveModel {
            id:                 Set(p.id.as_uuid()),
            supplier_id:        Set(p.supplier_id.as_uuid()),
            product_id:         Set(p.product_id),
            variant_id:         Set(p.variant_id),
            supplier_sku:       Set(p.supplier_sku.clone()),
            unit_cost:          Set(p.unit_cost),
            lead_time_days:     Set(p.lead_time_days),
            min_order_quantity: Set(p.min_order_quantity),
            is_preferred:       Set(p.is_preferred),
            created_at:         Set(from_utc(p.created_at)),
            created_by:         Set(p.created_by.clone()),
            updated_at:         Set(opt_from_utc(p.updated_at)),
            updated_by:         Set(p.updated_by.clone()),
        };
        supplier_product::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(supplier_product::Column::Id)
                    .update_columns([
                        supplier_product::Column::UnitCost,
                        supplier_product::Column::SupplierSku,
                        supplier_product::Column::LeadTimeDays,
                        supplier_product::Column::MinOrderQuantity,
                        supplier_product::Column::IsPreferred,
                        supplier_product::Column::UpdatedAt,
                        supplier_product::Column::UpdatedBy,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
    async fn delete(&self, id: SupplierProductId) -> Result<(), AppError> {
        supplier_product::Entity::delete_by_id(id.as_uuid()).exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
}

// ── PgPurchaseOrderRepository ─────────────────────────────────────────────────

pub struct PgPurchaseOrderRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl PurchaseOrderRepository for PgPurchaseOrderRepository {
    async fn find_by_id(&self, id: OrderId) -> Result<Option<PurchaseOrder>, AppError> {
        let Some(m) = purchase_order::Entity::find_by_id(id.as_uuid()).one(&*self.0).await.map_err(db_err)? else {
            return Ok(None);
        };
        let details = purchase_order_detail::Entity::find()
            .filter(purchase_order_detail::Column::OrderId.eq(id.as_uuid()))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(m2order_detail).collect();
        Ok(Some(m2order(m, details)))
    }

    async fn get_filtered(
        &self,
        supplier_id: Option<SupplierId>,
        store_id: Option<i32>,
        status: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<PurchaseOrder>, AppError> {
        let mut q = purchase_order::Entity::find();
        if let Some(sid) = supplier_id { q = q.filter(purchase_order::Column::SupplierId.eq(sid.as_uuid())); }
        if let Some(stid) = store_id   { q = q.filter(purchase_order::Column::StoreId.eq(stid)); }
        if let Some(st) = status       { q = q.filter(purchase_order::Column::Status.eq(st.to_string())); }
        if let Some(f)  = from         { q = q.filter(purchase_order::Column::OrderDate.gte(f.fixed_offset())); }
        if let Some(t)  = to           { q = q.filter(purchase_order::Column::OrderDate.lte(t.fixed_offset())); }
        let models = q.order_by_desc(purchase_order::Column::OrderDate).all(&*self.0).await.map_err(db_err)?;
        let mut result = Vec::with_capacity(models.len());
        for m in models {
            let oid = m.id;
            let details = purchase_order_detail::Entity::find()
                .filter(purchase_order_detail::Column::OrderId.eq(oid))
                .all(&*self.0).await.map_err(db_err)?
                .into_iter().map(m2order_detail).collect();
            result.push(m2order(m, details));
        }
        Ok(result)
    }

    async fn save(&self, o: &PurchaseOrder) -> Result<(), AppError> {
        let active = purchase_order::ActiveModel {
            id:                  Set(o.id.as_uuid()),
            supplier_id:         Set(o.supplier_id.as_uuid()),
            store_id:            Set(o.store_id),
            order_date:          Set(from_utc(o.order_date)),
            expected_date:       Set(opt_from_utc(o.expected_date)),
            status:              Set(o.status.to_string()),
            total_amount:        Set(o.total_amount),
            shipping_address_id: Set(o.shipping_address_id.map(|i| i.as_uuid())),
            contact_person_id:   Set(o.contact_person_id.map(|i| i.as_uuid())),
            created_at:          Set(from_utc(o.created_at)),
            created_by:          Set(o.created_by),
            updated_at:          Set(opt_from_utc(o.updated_at)),
            updated_by:          Set(o.updated_by),
        };
        purchase_order::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(purchase_order::Column::Id)
                    .update_columns([
                        purchase_order::Column::ExpectedDate,
                        purchase_order::Column::Status,
                        purchase_order::Column::TotalAmount,
                        purchase_order::Column::ShippingAddressId,
                        purchase_order::Column::ContactPersonId,
                        purchase_order::Column::UpdatedAt,
                        purchase_order::Column::UpdatedBy,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;

        // Replace details
        purchase_order_detail::Entity::delete_many()
            .filter(purchase_order_detail::Column::OrderId.eq(o.id.as_uuid()))
            .exec(&*self.0).await.map_err(db_err)?;

        if !o.order_details.is_empty() {
            let details: Vec<purchase_order_detail::ActiveModel> = o.order_details.iter().map(|d| {
                purchase_order_detail::ActiveModel {
                    id:                Set(d.id.as_uuid()),
                    order_id:          Set(d.order_id.as_uuid()),
                    product_id:        Set(d.product_id),
                    quantity:          Set(d.quantity),
                    unit_cost:         Set(d.unit_cost),
                    received_quantity: Set(d.received_quantity),
                    created_at:        Set(from_utc(d.created_at)),
                    created_by:        Set(d.created_by),
                }
            }).collect();
            purchase_order_detail::Entity::insert_many(details).exec(&*self.0).await.map_err(db_err)?;
        }
        Ok(())
    }
}
