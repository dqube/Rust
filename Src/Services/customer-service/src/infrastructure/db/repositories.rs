//! Postgres-backed implementations of the customer domain repositories.

use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ddd_shared_kernel::{AppError, AppResult, Page, PageRequest};
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use uuid::Uuid;

use super::models::*;
use crate::domain::entities::{
    Customer, CustomerAddress, CustomerContactNumber, CustomerProfile, KycDocument, WishlistItem,
};
use crate::domain::enums::{AddressType, ContactNumberType, Gender, KycStatus};
use crate::domain::ids::{
    ContactNumberId, CustomerAddressId, CustomerId, CustomerProfileId, WishlistItemId,
};
use crate::domain::repositories::{
    CustomerListFilter, CustomerProfileRepository, CustomerRepository, WishlistItemRepository,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

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
fn db_err(e: sea_orm::DbErr) -> AppError {
    AppError::internal(format!("db: {e}"))
}

// ── Mappers ──────────────────────────────────────────────────────────────────

fn model_to_customer(m: customer::Model) -> Customer {
    Customer {
        id: CustomerId::from_uuid(m.id),
        version: 0,
        created_at: to_utc(m.created_at),
        updated_at: opt_to_utc(m.updated_at).unwrap_or_else(|| to_utc(m.created_at)),
        domain_events: Vec::new(),
        user_id: m.user_id,
        first_name: m.first_name,
        last_name: m.last_name,
        email: m.email,
        membership_number: m.membership_number,
        join_date: to_utc(m.join_date),
        expiry_date: to_utc(m.expiry_date),
        country_code: m.country_code,
        loyalty_points: m.loyalty_points,
        preferred_contact_method: m.preferred_contact_method,
        preferred_address_type: m.preferred_address_type,
        created_by: m.created_by,
        updated_by: m.updated_by,
        contact_numbers: Vec::new(),
        addresses: Vec::new(),
    }
}

fn model_to_contact(m: contact_number::Model) -> CustomerContactNumber {
    CustomerContactNumber {
        id: ContactNumberId::from_uuid(m.id),
        customer_id: CustomerId::from_uuid(m.customer_id),
        contact_type: ContactNumberType::from_i16(m.contact_type),
        phone_number: m.phone_number,
        is_primary: m.is_primary,
        verified: m.verified,
        created_at: to_utc(m.created_at),
        updated_at: opt_to_utc(m.updated_at),
    }
}

fn model_to_address(m: address::Model) -> CustomerAddress {
    CustomerAddress {
        id: CustomerAddressId::from_uuid(m.id),
        customer_id: CustomerId::from_uuid(m.customer_id),
        label: m.label,
        address_type: AddressType::from_i16(m.address_type),
        line1: m.line1,
        line2: m.line2,
        city: m.city,
        state: m.state,
        postal_code: m.postal_code,
        country_code: m.country_code,
        is_primary: m.is_primary,
        created_at: to_utc(m.created_at),
        updated_at: opt_to_utc(m.updated_at),
    }
}

fn model_to_profile(m: profile::Model) -> AppResult<CustomerProfile> {
    let kyc_status = KycStatus::from_str(&m.kyc_status)?;
    let kyc_documents: Vec<KycDocument> = serde_json::from_value(m.kyc_documents)
        .map_err(|e| AppError::internal(format!("kyc_documents decode: {e}")))?;
    Ok(CustomerProfile {
        id: CustomerProfileId::from_uuid(m.id),
        version: 0,
        created_at: to_utc(m.created_at),
        updated_at: opt_to_utc(m.updated_at).unwrap_or_else(|| to_utc(m.created_at)),
        domain_events: Vec::new(),
        customer_id: CustomerId::from_uuid(m.customer_id),
        date_of_birth: opt_to_utc(m.date_of_birth),
        gender: m.gender.map(Gender::from_i16),
        preferred_language: m.preferred_language,
        preferred_currency: m.preferred_currency,
        tax_id: m.tax_id,
        company_registration_number: m.company_registration_number,
        email_notifications: m.email_notifications,
        sms_notifications: m.sms_notifications,
        push_notifications: m.push_notifications,
        marketing_emails: m.marketing_emails,
        kyc_status,
        kyc_verified_at: opt_to_utc(m.kyc_verified_at),
        kyc_documents,
        avatar_object_name: m.avatar_object_name,
        created_by: m.created_by,
        updated_by: m.updated_by,
    })
}

fn model_to_wishlist_item(m: wishlist_item::Model) -> WishlistItem {
    use sea_orm::prelude::Decimal;
    let base_price = <f64 as TryFrom<Decimal>>::try_from(m.base_price).unwrap_or(0.0);
    WishlistItem {
        id: WishlistItemId::from_uuid(m.id),
        customer_id: CustomerId::from_uuid(m.customer_id),
        product_id: m.product_id,
        product_name: m.product_name,
        base_price,
        added_at: to_utc(m.added_at),
        domain_events: Vec::new(),
    }
}

// ── PgCustomerRepository ─────────────────────────────────────────────────────

pub struct PgCustomerRepository(pub Arc<DatabaseConnection>);

async fn load_children(
    db: &DatabaseConnection,
    customer: &mut Customer,
) -> AppResult<()> {
    let contacts = contact_number::Entity::find()
        .filter(contact_number::Column::CustomerId.eq(customer.id.as_uuid()))
        .order_by_asc(contact_number::Column::CreatedAt)
        .all(db)
        .await
        .map_err(db_err)?;
    customer.contact_numbers = contacts.into_iter().map(model_to_contact).collect();

    let addrs = address::Entity::find()
        .filter(address::Column::CustomerId.eq(customer.id.as_uuid()))
        .order_by_asc(address::Column::CreatedAt)
        .all(db)
        .await
        .map_err(db_err)?;
    customer.addresses = addrs.into_iter().map(model_to_address).collect();
    Ok(())
}

#[async_trait]
impl CustomerRepository for PgCustomerRepository {
    async fn find_by_id(&self, id: CustomerId) -> AppResult<Option<Customer>> {
        Ok(customer::Entity::find_by_id(id.as_uuid())
            .one(&*self.0)
            .await
            .map_err(db_err)?
            .map(model_to_customer))
    }

    async fn find_with_children(&self, id: CustomerId) -> AppResult<Option<Customer>> {
        let Some(m) = customer::Entity::find_by_id(id.as_uuid())
            .one(&*self.0)
            .await
            .map_err(db_err)?
        else {
            return Ok(None);
        };
        let mut c = model_to_customer(m);
        load_children(&self.0, &mut c).await?;
        Ok(Some(c))
    }

    async fn find_by_user_id(&self, user_id: Uuid) -> AppResult<Option<Customer>> {
        Ok(customer::Entity::find()
            .filter(customer::Column::UserId.eq(user_id))
            .one(&*self.0)
            .await
            .map_err(db_err)?
            .map(model_to_customer))
    }

    async fn find_by_email(&self, email: &str) -> AppResult<Option<Customer>> {
        Ok(customer::Entity::find()
            .filter(customer::Column::Email.eq(email))
            .one(&*self.0)
            .await
            .map_err(db_err)?
            .map(model_to_customer))
    }

    async fn email_exists(&self, email: &str) -> AppResult<bool> {
        Ok(customer::Entity::find()
            .filter(customer::Column::Email.eq(email))
            .count(&*self.0)
            .await
            .map_err(db_err)?
            > 0)
    }

    async fn membership_number_exists(&self, number: &str) -> AppResult<bool> {
        Ok(customer::Entity::find()
            .filter(customer::Column::MembershipNumber.eq(number))
            .count(&*self.0)
            .await
            .map_err(db_err)?
            > 0)
    }

    async fn list_paged(
        &self,
        page: PageRequest,
        filter: CustomerListFilter,
    ) -> AppResult<Page<Customer>> {
        let mut select = customer::Entity::find().order_by_asc(customer::Column::CreatedAt);

        if let Some(ref s) = filter.search {
            if !s.is_empty() {
                let like = format!("%{}%", s.to_lowercase());
                select = select.filter(
                    Condition::any()
                        .add(customer::Column::FirstName.contains(like.clone()))
                        .add(customer::Column::LastName.contains(like.clone()))
                        .add(customer::Column::Email.contains(like)),
                );
            }
        }
        if let Some(ref cc) = filter.country_code {
            if !cc.is_empty() {
                select = select.filter(customer::Column::CountryCode.eq(cc.to_uppercase()));
            }
        }
        if let Some(active) = filter.is_membership_active {
            let now = Utc::now().fixed_offset();
            select = if active {
                select.filter(customer::Column::ExpiryDate.gt(now))
            } else {
                select.filter(customer::Column::ExpiryDate.lte(now))
            };
        }

        let per_page = u64::from(page.per_page().max(1));
        let page_num = u64::from(page.page().max(1));
        let paginator = select.paginate(&*self.0, per_page);
        let total = paginator.num_items().await.map_err(db_err)?;
        let rows = paginator.fetch_page(page_num - 1).await.map_err(db_err)?;

        let mut customers: Vec<Customer> = rows.into_iter().map(model_to_customer).collect();
        for c in &mut customers {
            load_children(&self.0, c).await?;
        }
        Ok(Page::new(customers, total, page.page(), page.per_page()))
    }

    async fn save(&self, c: &Customer) -> AppResult<()> {
        let txn = self.0.begin().await.map_err(db_err)?;

        let active = customer::ActiveModel {
            id: Set(c.id.as_uuid()),
            user_id: Set(c.user_id),
            first_name: Set(c.first_name.clone()),
            last_name: Set(c.last_name.clone()),
            email: Set(c.email.clone()),
            membership_number: Set(c.membership_number.clone()),
            join_date: Set(from_utc(c.join_date)),
            expiry_date: Set(from_utc(c.expiry_date)),
            country_code: Set(c.country_code.clone()),
            loyalty_points: Set(c.loyalty_points),
            preferred_contact_method: Set(c.preferred_contact_method),
            preferred_address_type: Set(c.preferred_address_type),
            created_at: Set(from_utc(c.created_at)),
            created_by: Set(c.created_by.clone()),
            updated_at: Set(opt_from_utc(c.updated_at)),
            updated_by: Set(c.updated_by.clone()),
        };
        customer::Entity::insert(active)
            .on_conflict(
                OnConflict::column(customer::Column::Id)
                    .update_columns([
                        customer::Column::FirstName,
                        customer::Column::LastName,
                        customer::Column::Email,
                        customer::Column::MembershipNumber,
                        customer::Column::JoinDate,
                        customer::Column::ExpiryDate,
                        customer::Column::CountryCode,
                        customer::Column::LoyaltyPoints,
                        customer::Column::PreferredContactMethod,
                        customer::Column::PreferredAddressType,
                        customer::Column::UpdatedAt,
                        customer::Column::UpdatedBy,
                    ])
                    .to_owned(),
            )
            .exec(&txn)
            .await
            .map_err(db_err)?;

        // ── Reconcile contact numbers ────────────────────────────────
        let existing_contact_ids: Vec<Uuid> = contact_number::Entity::find()
            .filter(contact_number::Column::CustomerId.eq(c.id.as_uuid()))
            .all(&txn)
            .await
            .map_err(db_err)?
            .into_iter()
            .map(|m| m.id)
            .collect();
        let keep_contact_ids: Vec<Uuid> = c.contact_numbers.iter().map(|n| n.id.as_uuid()).collect();
        let remove_contacts: Vec<Uuid> = existing_contact_ids
            .into_iter()
            .filter(|id| !keep_contact_ids.contains(id))
            .collect();
        if !remove_contacts.is_empty() {
            contact_number::Entity::delete_many()
                .filter(contact_number::Column::Id.is_in(remove_contacts))
                .exec(&txn)
                .await
                .map_err(db_err)?;
        }
        for n in &c.contact_numbers {
            let active = contact_number::ActiveModel {
                id: Set(n.id.as_uuid()),
                customer_id: Set(c.id.as_uuid()),
                contact_type: Set(n.contact_type.to_i16()),
                phone_number: Set(n.phone_number.clone()),
                is_primary: Set(n.is_primary),
                verified: Set(n.verified),
                created_at: Set(from_utc(n.created_at)),
                updated_at: Set(opt_from_utc(n.updated_at)),
            };
            contact_number::Entity::insert(active)
                .on_conflict(
                    OnConflict::column(contact_number::Column::Id)
                        .update_columns([
                            contact_number::Column::ContactType,
                            contact_number::Column::PhoneNumber,
                            contact_number::Column::IsPrimary,
                            contact_number::Column::Verified,
                            contact_number::Column::UpdatedAt,
                        ])
                        .to_owned(),
                )
                .exec(&txn)
                .await
                .map_err(db_err)?;
        }

        // ── Reconcile addresses ──────────────────────────────────────
        let existing_addr_ids: Vec<Uuid> = address::Entity::find()
            .filter(address::Column::CustomerId.eq(c.id.as_uuid()))
            .all(&txn)
            .await
            .map_err(db_err)?
            .into_iter()
            .map(|m| m.id)
            .collect();
        let keep_addr_ids: Vec<Uuid> = c.addresses.iter().map(|a| a.id.as_uuid()).collect();
        let remove_addrs: Vec<Uuid> = existing_addr_ids
            .into_iter()
            .filter(|id| !keep_addr_ids.contains(id))
            .collect();
        if !remove_addrs.is_empty() {
            address::Entity::delete_many()
                .filter(address::Column::Id.is_in(remove_addrs))
                .exec(&txn)
                .await
                .map_err(db_err)?;
        }
        for a in &c.addresses {
            let active = address::ActiveModel {
                id: Set(a.id.as_uuid()),
                customer_id: Set(c.id.as_uuid()),
                label: Set(a.label.clone()),
                address_type: Set(a.address_type.to_i16()),
                line1: Set(a.line1.clone()),
                line2: Set(a.line2.clone()),
                city: Set(a.city.clone()),
                state: Set(a.state.clone()),
                postal_code: Set(a.postal_code.clone()),
                country_code: Set(a.country_code.clone()),
                is_primary: Set(a.is_primary),
                created_at: Set(from_utc(a.created_at)),
                updated_at: Set(opt_from_utc(a.updated_at)),
            };
            address::Entity::insert(active)
                .on_conflict(
                    OnConflict::column(address::Column::Id)
                        .update_columns([
                            address::Column::Label,
                            address::Column::AddressType,
                            address::Column::Line1,
                            address::Column::Line2,
                            address::Column::City,
                            address::Column::State,
                            address::Column::PostalCode,
                            address::Column::CountryCode,
                            address::Column::IsPrimary,
                            address::Column::UpdatedAt,
                        ])
                        .to_owned(),
                )
                .exec(&txn)
                .await
                .map_err(db_err)?;
        }

        txn.commit().await.map_err(db_err)?;
        Ok(())
    }
}

// ── PgCustomerProfileRepository ──────────────────────────────────────────────

pub struct PgCustomerProfileRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl CustomerProfileRepository for PgCustomerProfileRepository {
    async fn find_by_id(&self, id: CustomerProfileId) -> AppResult<Option<CustomerProfile>> {
        match profile::Entity::find_by_id(id.as_uuid())
            .one(&*self.0)
            .await
            .map_err(db_err)?
        {
            Some(m) => Ok(Some(model_to_profile(m)?)),
            None => Ok(None),
        }
    }

    async fn find_by_customer_id(
        &self,
        customer_id: CustomerId,
    ) -> AppResult<Option<CustomerProfile>> {
        match profile::Entity::find()
            .filter(profile::Column::CustomerId.eq(customer_id.as_uuid()))
            .one(&*self.0)
            .await
            .map_err(db_err)?
        {
            Some(m) => Ok(Some(model_to_profile(m)?)),
            None => Ok(None),
        }
    }

    async fn exists_by_customer_id(&self, customer_id: CustomerId) -> AppResult<bool> {
        Ok(profile::Entity::find()
            .filter(profile::Column::CustomerId.eq(customer_id.as_uuid()))
            .count(&*self.0)
            .await
            .map_err(db_err)?
            > 0)
    }

    async fn list_by_kyc_status(
        &self,
        status: KycStatus,
        page: PageRequest,
    ) -> AppResult<Page<CustomerProfile>> {
        let select = profile::Entity::find()
            .filter(profile::Column::KycStatus.eq(status.to_string()))
            .order_by_asc(profile::Column::CreatedAt);
        let per_page = u64::from(page.per_page().max(1));
        let page_num = u64::from(page.page().max(1));
        let paginator = select.paginate(&*self.0, per_page);
        let total = paginator.num_items().await.map_err(db_err)?;
        let rows = paginator.fetch_page(page_num - 1).await.map_err(db_err)?;
        let mut items = Vec::with_capacity(rows.len());
        for r in rows {
            items.push(model_to_profile(r)?);
        }
        Ok(Page::new(items, total, page.page(), page.per_page()))
    }

    async fn save(&self, p: &CustomerProfile) -> AppResult<()> {
        let documents = serde_json::to_value(&p.kyc_documents)
            .map_err(|e| AppError::internal(format!("kyc_documents encode: {e}")))?;
        let active = profile::ActiveModel {
            id: Set(p.id.as_uuid()),
            customer_id: Set(p.customer_id.as_uuid()),
            date_of_birth: Set(opt_from_utc(p.date_of_birth)),
            gender: Set(p.gender.map(|g| g.to_i16())),
            preferred_language: Set(p.preferred_language.clone()),
            preferred_currency: Set(p.preferred_currency.clone()),
            tax_id: Set(p.tax_id.clone()),
            company_registration_number: Set(p.company_registration_number.clone()),
            email_notifications: Set(p.email_notifications),
            sms_notifications: Set(p.sms_notifications),
            push_notifications: Set(p.push_notifications),
            marketing_emails: Set(p.marketing_emails),
            kyc_status: Set(p.kyc_status.to_string()),
            kyc_verified_at: Set(opt_from_utc(p.kyc_verified_at)),
            kyc_documents: Set(documents),
            avatar_object_name: Set(p.avatar_object_name.clone()),
            created_at: Set(from_utc(p.created_at)),
            created_by: Set(p.created_by.clone()),
            updated_at: Set(opt_from_utc(p.updated_at)),
            updated_by: Set(p.updated_by.clone()),
        };
        profile::Entity::insert(active)
            .on_conflict(
                OnConflict::column(profile::Column::Id)
                    .update_columns([
                        profile::Column::DateOfBirth,
                        profile::Column::Gender,
                        profile::Column::PreferredLanguage,
                        profile::Column::PreferredCurrency,
                        profile::Column::TaxId,
                        profile::Column::CompanyRegistrationNumber,
                        profile::Column::EmailNotifications,
                        profile::Column::SmsNotifications,
                        profile::Column::PushNotifications,
                        profile::Column::MarketingEmails,
                        profile::Column::KycStatus,
                        profile::Column::KycVerifiedAt,
                        profile::Column::KycDocuments,
                        profile::Column::AvatarObjectName,
                        profile::Column::UpdatedAt,
                        profile::Column::UpdatedBy,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0)
            .await
            .map_err(db_err)?;
        Ok(())
    }
}

// ── PgWishlistItemRepository ─────────────────────────────────────────────────

pub struct PgWishlistItemRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl WishlistItemRepository for PgWishlistItemRepository {
    async fn find_by_id(&self, id: WishlistItemId) -> AppResult<Option<WishlistItem>> {
        Ok(wishlist_item::Entity::find_by_id(id.as_uuid())
            .one(&*self.0)
            .await
            .map_err(db_err)?
            .map(model_to_wishlist_item))
    }

    async fn find_by_customer_and_product(
        &self,
        customer_id: CustomerId,
        product_id: Uuid,
    ) -> AppResult<Option<WishlistItem>> {
        Ok(wishlist_item::Entity::find()
            .filter(wishlist_item::Column::CustomerId.eq(customer_id.as_uuid()))
            .filter(wishlist_item::Column::ProductId.eq(product_id))
            .one(&*self.0)
            .await
            .map_err(db_err)?
            .map(model_to_wishlist_item))
    }

    async fn list_by_customer_id(
        &self,
        customer_id: CustomerId,
    ) -> AppResult<Vec<WishlistItem>> {
        Ok(wishlist_item::Entity::find()
            .filter(wishlist_item::Column::CustomerId.eq(customer_id.as_uuid()))
            .order_by_desc(wishlist_item::Column::AddedAt)
            .all(&*self.0)
            .await
            .map_err(db_err)?
            .into_iter()
            .map(model_to_wishlist_item)
            .collect())
    }

    async fn save(&self, item: &WishlistItem) -> AppResult<()> {
        use sea_orm::prelude::Decimal;
        let price = Decimal::try_from(item.base_price).unwrap_or_default();
        let active = wishlist_item::ActiveModel {
            id: Set(item.id.as_uuid()),
            customer_id: Set(item.customer_id.as_uuid()),
            product_id: Set(item.product_id),
            product_name: Set(item.product_name.clone()),
            base_price: Set(price),
            added_at: Set(from_utc(item.added_at)),
        };
        wishlist_item::Entity::insert(active)
            .on_conflict(
                OnConflict::columns([
                    wishlist_item::Column::CustomerId,
                    wishlist_item::Column::ProductId,
                ])
                .update_columns([
                    wishlist_item::Column::ProductName,
                    wishlist_item::Column::BasePrice,
                ])
                .to_owned(),
            )
            .exec(&*self.0)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn delete(&self, id: WishlistItemId) -> AppResult<()> {
        wishlist_item::Entity::delete_by_id(id.as_uuid())
            .exec(&*self.0)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn delete_all_by_customer_id(&self, customer_id: CustomerId) -> AppResult<()> {
        wishlist_item::Entity::delete_many()
            .filter(wishlist_item::Column::CustomerId.eq(customer_id.as_uuid()))
            .exec(&*self.0)
            .await
            .map_err(db_err)?;
        Ok(())
    }
}

