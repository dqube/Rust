//! Customer aggregates: Customer (with addresses + contact numbers),
//! CustomerProfile (with KYC documents), and WishlistItem.

use chrono::{DateTime, Datelike, Utc};
use ddd_shared_kernel::{AppError, AppResult, DomainEvent};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::enums::{AddressType, ContactNumberType, Gender, KycStatus};
use super::events::{
    CustomerCreated, CustomerEmailUpdated, KycRejected, KycVerified, LoyaltyPointsUpdated,
    WishlistItemAdded, WishlistItemRemoved,
};
use super::ids::{
    ContactNumberId, CustomerAddressId, CustomerId, CustomerProfileId, WishlistItemId,
};

// ── CustomerContactNumber ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CustomerContactNumber {
    pub id: ContactNumberId,
    pub customer_id: CustomerId,
    pub contact_type: ContactNumberType,
    pub phone_number: String,
    pub is_primary: bool,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl CustomerContactNumber {
    pub fn new(
        id: ContactNumberId,
        customer_id: CustomerId,
        contact_type: ContactNumberType,
        phone_number: String,
        is_primary: bool,
    ) -> Self {
        Self {
            id,
            customer_id,
            contact_type,
            phone_number,
            is_primary,
            verified: false,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    pub fn set_as_primary(&mut self, is_primary: bool) {
        self.is_primary = is_primary;
        self.updated_at = Some(Utc::now());
    }

    pub fn mark_as_verified(&mut self) {
        self.verified = true;
        self.updated_at = Some(Utc::now());
    }

    pub fn update_phone_number(&mut self, phone_number: String) {
        self.phone_number = phone_number;
        self.verified = false;
        self.updated_at = Some(Utc::now());
    }
}

// ── CustomerAddress ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CustomerAddress {
    pub id: CustomerAddressId,
    pub customer_id: CustomerId,
    pub label: String,
    pub address_type: AddressType,
    pub line1: String,
    pub line2: Option<String>,
    pub city: String,
    pub state: Option<String>,
    pub postal_code: String,
    pub country_code: String,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl CustomerAddress {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: CustomerAddressId,
        customer_id: CustomerId,
        label: String,
        address_type: AddressType,
        line1: String,
        line2: Option<String>,
        city: String,
        state: Option<String>,
        postal_code: String,
        country_code: String,
        is_primary: bool,
    ) -> AppResult<Self> {
        let label = if label.trim().is_empty() {
            address_type.as_str().to_owned()
        } else {
            label
        };
        let addr = Self {
            id,
            customer_id,
            label,
            address_type,
            line1,
            line2,
            city,
            state,
            postal_code,
            country_code,
            is_primary,
            created_at: Utc::now(),
            updated_at: None,
        };
        addr.validate()?;
        Ok(addr)
    }

    pub fn update(
        &mut self,
        label: String,
        line1: String,
        line2: Option<String>,
        city: String,
        state: Option<String>,
        postal_code: String,
        country_code: String,
    ) -> AppResult<()> {
        self.label = if label.trim().is_empty() {
            self.address_type.as_str().to_owned()
        } else {
            label
        };
        self.line1 = line1;
        self.line2 = line2;
        self.city = city;
        self.state = state;
        self.postal_code = postal_code;
        self.country_code = country_code;
        self.updated_at = Some(Utc::now());
        self.validate()
    }

    pub fn set_as_primary(&mut self, is_primary: bool) {
        self.is_primary = is_primary;
        self.updated_at = Some(Utc::now());
    }

    fn validate(&self) -> AppResult<()> {
        if self.line1.trim().is_empty() || self.line1.len() > 200 {
            return Err(AppError::validation("line1", "must be 1-200 characters"));
        }
        if self.line2.as_ref().is_some_and(|l| l.len() > 200) {
            return Err(AppError::validation("line2", "must be <= 200 characters"));
        }
        if self.city.trim().is_empty() || self.city.len() > 100 {
            return Err(AppError::validation("city", "must be 1-100 characters"));
        }
        if self.state.as_ref().is_some_and(|s| s.len() > 100) {
            return Err(AppError::validation("state", "must be <= 100 characters"));
        }
        if self.postal_code.trim().is_empty() || self.postal_code.len() > 20 {
            return Err(AppError::validation(
                "postal_code",
                "must be 1-20 characters",
            ));
        }
        if self.country_code.len() != 2 && self.country_code.len() != 3 {
            return Err(AppError::validation(
                "country_code",
                "must be a 2- or 3-character ISO code",
            ));
        }
        Ok(())
    }
}

// ── Customer ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct Customer {
    pub id: CustomerId,
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub membership_number: String,
    pub join_date: DateTime<Utc>,
    pub expiry_date: DateTime<Utc>,
    pub country_code: String,
    pub loyalty_points: i32,
    pub preferred_contact_method: Option<i32>,
    pub preferred_address_type: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub updated_by: Option<String>,
    pub contact_numbers: Vec<CustomerContactNumber>,
    pub addresses: Vec<CustomerAddress>,
    #[doc(hidden)]
    pub domain_events: Vec<Box<dyn DomainEvent>>,
}

impl Customer {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        id: CustomerId,
        user_id: Uuid,
        first_name: String,
        last_name: String,
        email: Option<String>,
        membership_number: String,
        join_date: DateTime<Utc>,
        expiry_date: DateTime<Utc>,
        country_code: String,
    ) -> AppResult<Self> {
        if user_id.is_nil() {
            return Err(AppError::validation("user_id", "must not be nil"));
        }
        let now = Utc::now();
        let mut c = Self {
            id,
            user_id,
            first_name: first_name.clone(),
            last_name: last_name.clone(),
            email: email.clone(),
            membership_number: membership_number.clone(),
            join_date,
            expiry_date,
            country_code: country_code.clone(),
            loyalty_points: 0,
            preferred_contact_method: None,
            preferred_address_type: None,
            created_at: now,
            created_by: None,
            updated_at: None,
            updated_by: None,
            contact_numbers: Vec::new(),
            addresses: Vec::new(),
            domain_events: Vec::new(),
        };
        c.validate()?;
        c.domain_events.push(Box::new(CustomerCreated {
            customer_id: id,
            user_id,
            first_name,
            last_name,
            email,
            membership_number,
            join_date,
            occurred_at: now,
        }));
        Ok(c)
    }

    pub fn update_email(&mut self, new_email: Option<String>) {
        let old = self.email.clone();
        if old != new_email {
            self.email = new_email.clone();
            self.updated_at = Some(Utc::now());
            self.domain_events.push(Box::new(CustomerEmailUpdated {
                customer_id: self.id,
                old_email: old,
                new_email,
                occurred_at: Utc::now(),
            }));
        }
    }

    pub fn update_personal_info(&mut self, first_name: String, last_name: String) -> AppResult<()> {
        self.first_name = first_name;
        self.last_name = last_name;
        self.updated_at = Some(Utc::now());
        self.validate_personal_info()
    }

    pub fn add_loyalty_points(&mut self, points: i32, reason: String) -> AppResult<()> {
        if points <= 0 {
            return Err(AppError::validation("points", "must be positive"));
        }
        let previous = self.loyalty_points;
        self.loyalty_points += points;
        self.updated_at = Some(Utc::now());
        self.domain_events.push(Box::new(LoyaltyPointsUpdated {
            customer_id: self.id,
            previous_points: previous,
            new_points: self.loyalty_points,
            reason,
            occurred_at: Utc::now(),
        }));
        Ok(())
    }

    pub fn redeem_loyalty_points(&mut self, points: i32, reason: String) -> AppResult<()> {
        if points <= 0 {
            return Err(AppError::validation("points", "must be positive"));
        }
        if self.loyalty_points < points {
            return Err(AppError::conflict(
                "customer does not have enough loyalty points",
            ));
        }
        let previous = self.loyalty_points;
        self.loyalty_points -= points;
        self.updated_at = Some(Utc::now());
        self.domain_events.push(Box::new(LoyaltyPointsUpdated {
            customer_id: self.id,
            previous_points: previous,
            new_points: self.loyalty_points,
            reason,
            occurred_at: Utc::now(),
        }));
        Ok(())
    }

    pub fn add_contact_number(&mut self, contact: CustomerContactNumber) {
        if contact.is_primary {
            for c in &mut self.contact_numbers {
                c.set_as_primary(false);
            }
        }
        self.contact_numbers.push(contact);
    }

    pub fn set_primary_contact_number(&mut self, contact_type: ContactNumberType, phone: String) {
        if let Some(existing) = self
            .contact_numbers
            .iter_mut()
            .find(|c| c.contact_type == contact_type)
        {
            existing.update_phone_number(phone.clone());
            let id = existing.id;
            for c in &mut self.contact_numbers {
                c.set_as_primary(c.id == id);
            }
        } else {
            for c in &mut self.contact_numbers {
                c.set_as_primary(false);
            }
            self.contact_numbers.push(CustomerContactNumber::new(
                ContactNumberId::new(),
                self.id,
                contact_type,
                phone,
                true,
            ));
        }
    }

    pub fn add_address(&mut self, address: CustomerAddress) {
        if address.is_primary {
            for a in &mut self.addresses {
                a.set_as_primary(false);
            }
        }
        self.addresses.push(address);
    }

    pub fn remove_address(&mut self, address_id: CustomerAddressId) -> AppResult<()> {
        let pos = self
            .addresses
            .iter()
            .position(|a| a.id == address_id)
            .ok_or_else(|| AppError::not_found("CustomerAddress", address_id.to_string()))?;
        self.addresses.remove(pos);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_address(
        &mut self,
        address_id: CustomerAddressId,
        label: String,
        line1: String,
        line2: Option<String>,
        city: String,
        state: Option<String>,
        postal_code: String,
        country_code: String,
    ) -> AppResult<()> {
        let addr = self
            .addresses
            .iter_mut()
            .find(|a| a.id == address_id)
            .ok_or_else(|| AppError::not_found("CustomerAddress", address_id.to_string()))?;
        addr.update(label, line1, line2, city, state, postal_code, country_code)
    }

    pub fn set_default_address(&mut self, address_id: CustomerAddressId) -> AppResult<()> {
        if !self.addresses.iter().any(|a| a.id == address_id) {
            return Err(AppError::not_found(
                "CustomerAddress",
                address_id.to_string(),
            ));
        }
        for a in &mut self.addresses {
            a.set_as_primary(a.id == address_id);
        }
        Ok(())
    }

    pub fn is_membership_active(&self) -> bool {
        self.expiry_date > Utc::now()
    }

    pub fn primary_phone(&self) -> Option<&str> {
        self.contact_numbers
            .iter()
            .find(|c| c.is_primary)
            .map(|c| c.phone_number.as_str())
    }

    pub fn drain_events(&mut self) -> Vec<Box<dyn DomainEvent>> {
        std::mem::take(&mut self.domain_events)
    }

    fn validate(&self) -> AppResult<()> {
        self.validate_personal_info()?;
        if self.expiry_date <= self.join_date {
            return Err(AppError::validation(
                "expiry_date",
                "must be after join_date",
            ));
        }
        if self.country_code.len() != 2 && self.country_code.len() != 3 {
            return Err(AppError::validation(
                "country_code",
                "must be a 2- or 3-character ISO code",
            ));
        }
        Ok(())
    }

    fn validate_personal_info(&self) -> AppResult<()> {
        if self.first_name.trim().is_empty() || self.first_name.len() > 100 {
            return Err(AppError::validation(
                "first_name",
                "must be 1-100 characters",
            ));
        }
        if self.last_name.len() > 100 {
            return Err(AppError::validation(
                "last_name",
                "must be at most 100 characters",
            ));
        }
        Ok(())
    }
}

// ── KycDocument (value object persisted as JSONB) ────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycDocument {
    pub document_type: String,
    pub document_number: String,
    pub file_url: String,
    pub uploaded_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
}

impl KycDocument {
    pub fn new(document_type: String, document_number: String, file_url: String) -> AppResult<Self> {
        if document_type.trim().is_empty() {
            return Err(AppError::validation("document_type", "must not be empty"));
        }
        if document_number.trim().is_empty() {
            return Err(AppError::validation("document_number", "must not be empty"));
        }
        if file_url.trim().is_empty() {
            return Err(AppError::validation("file_url", "must not be empty"));
        }
        Ok(Self {
            document_type,
            document_number,
            file_url,
            uploaded_at: Utc::now(),
            verified_at: None,
            rejection_reason: None,
        })
    }

    pub fn mark_as_verified(&mut self) {
        self.verified_at = Some(Utc::now());
        self.rejection_reason = None;
    }

    pub fn mark_as_rejected(&mut self, reason: String) {
        self.rejection_reason = Some(reason);
        self.verified_at = None;
    }
}

// ── CustomerProfile ─────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct CustomerProfile {
    pub id: CustomerProfileId,
    pub customer_id: CustomerId,
    pub date_of_birth: Option<DateTime<Utc>>,
    pub gender: Option<Gender>,
    pub preferred_language: String,
    pub preferred_currency: String,
    pub tax_id: Option<String>,
    pub company_registration_number: Option<String>,
    pub email_notifications: bool,
    pub sms_notifications: bool,
    pub push_notifications: bool,
    pub marketing_emails: bool,
    pub kyc_status: KycStatus,
    pub kyc_verified_at: Option<DateTime<Utc>>,
    pub kyc_documents: Vec<KycDocument>,
    pub avatar_object_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub updated_by: Option<String>,
    #[doc(hidden)]
    pub domain_events: Vec<Box<dyn DomainEvent>>,
}

impl CustomerProfile {
    pub fn new(
        id: CustomerProfileId,
        customer_id: CustomerId,
        preferred_language: String,
        preferred_currency: String,
    ) -> AppResult<Self> {
        let lang = if preferred_language.is_empty() {
            "en".to_owned()
        } else {
            preferred_language
        };
        let curr = if preferred_currency.is_empty() {
            "USD".to_owned()
        } else {
            preferred_currency
        };
        if lang.len() != 2 {
            return Err(AppError::validation(
                "preferred_language",
                "must be 2 characters (ISO 639-1)",
            ));
        }
        if curr.len() != 3 {
            return Err(AppError::validation(
                "preferred_currency",
                "must be 3 characters (ISO 4217)",
            ));
        }
        Ok(Self {
            id,
            customer_id,
            date_of_birth: None,
            gender: None,
            preferred_language: lang,
            preferred_currency: curr,
            tax_id: None,
            company_registration_number: None,
            email_notifications: true,
            sms_notifications: true,
            push_notifications: true,
            marketing_emails: false,
            kyc_status: KycStatus::Pending,
            kyc_verified_at: None,
            kyc_documents: Vec::new(),
            avatar_object_name: None,
            created_at: Utc::now(),
            created_by: None,
            updated_at: None,
            updated_by: None,
            domain_events: Vec::new(),
        })
    }

    pub fn update_date_of_birth(&mut self, dob: Option<DateTime<Utc>>) -> AppResult<()> {
        if let Some(dob) = dob {
            Self::validate_dob(dob)?;
        }
        self.date_of_birth = dob;
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn update_gender(&mut self, gender: Option<Gender>) {
        self.gender = gender;
        self.updated_at = Some(Utc::now());
    }

    pub fn update_preferred_language(&mut self, lang: String) -> AppResult<()> {
        if lang.len() != 2 {
            return Err(AppError::validation(
                "preferred_language",
                "must be 2 characters",
            ));
        }
        self.preferred_language = lang;
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn update_preferred_currency(&mut self, curr: String) -> AppResult<()> {
        if curr.len() != 3 {
            return Err(AppError::validation(
                "preferred_currency",
                "must be 3 characters",
            ));
        }
        self.preferred_currency = curr;
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn update_tax_id(&mut self, tax_id: Option<String>) -> AppResult<()> {
        if let Some(ref id) = tax_id {
            if id.len() > 50 {
                return Err(AppError::validation(
                    "tax_id",
                    "must be at most 50 characters",
                ));
            }
        }
        self.tax_id = tax_id;
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn update_company_registration(&mut self, reg: Option<String>) -> AppResult<()> {
        if let Some(ref r) = reg {
            if r.len() > 100 {
                return Err(AppError::validation(
                    "company_registration_number",
                    "must be at most 100 characters",
                ));
            }
        }
        self.company_registration_number = reg;
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn update_notification_preferences(
        &mut self,
        email: bool,
        sms: bool,
        push: bool,
        marketing: bool,
    ) {
        self.email_notifications = email;
        self.sms_notifications = sms;
        self.push_notifications = push;
        self.marketing_emails = marketing;
        self.updated_at = Some(Utc::now());
    }

    pub fn submit_kyc_document(&mut self, doc: KycDocument) {
        self.kyc_documents.push(doc);
        self.updated_at = Some(Utc::now());
    }

    pub fn submit_for_kyc_review(&mut self) -> AppResult<()> {
        if self.kyc_documents.is_empty() {
            return Err(AppError::conflict(
                "cannot submit for KYC review without documents",
            ));
        }
        if self.kyc_status == KycStatus::Verified {
            return Err(AppError::conflict("KYC is already verified"));
        }
        self.kyc_status = KycStatus::Submitted;
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn verify_kyc(&mut self) -> AppResult<()> {
        if self.kyc_status == KycStatus::Verified {
            return Err(AppError::conflict("KYC is already verified"));
        }
        self.kyc_status = KycStatus::Verified;
        let verified_at = Utc::now();
        self.kyc_verified_at = Some(verified_at);
        self.updated_at = Some(verified_at);
        for doc in &mut self.kyc_documents {
            doc.mark_as_verified();
        }
        self.domain_events.push(Box::new(KycVerified {
            customer_id: self.customer_id,
            verified_at,
            occurred_at: verified_at,
        }));
        Ok(())
    }

    pub fn reject_kyc(&mut self, reason: String) -> AppResult<()> {
        if reason.trim().is_empty() {
            return Err(AppError::validation(
                "rejection_reason",
                "must not be empty",
            ));
        }
        self.kyc_status = KycStatus::Rejected;
        self.kyc_verified_at = None;
        let now = Utc::now();
        self.updated_at = Some(now);
        for doc in &mut self.kyc_documents {
            doc.mark_as_rejected(reason.clone());
        }
        self.domain_events.push(Box::new(KycRejected {
            customer_id: self.customer_id,
            rejection_reason: reason,
            occurred_at: now,
        }));
        Ok(())
    }

    pub fn set_avatar(&mut self, object_name: String) {
        self.avatar_object_name = Some(object_name);
        self.updated_at = Some(Utc::now());
    }

    pub fn remove_avatar(&mut self) {
        self.avatar_object_name = None;
        self.updated_at = Some(Utc::now());
    }

    pub fn is_kyc_verified(&self) -> bool {
        self.kyc_status == KycStatus::Verified
    }

    pub fn get_age(&self) -> Option<i32> {
        let dob = self.date_of_birth?;
        let today = Utc::now();
        let mut age = today.year() - dob.year();
        if today.ordinal() < dob.ordinal() {
            age -= 1;
        }
        Some(age)
    }

    pub fn drain_events(&mut self) -> Vec<Box<dyn DomainEvent>> {
        std::mem::take(&mut self.domain_events)
    }

    fn validate_dob(dob: DateTime<Utc>) -> AppResult<()> {
        if dob >= Utc::now() {
            return Err(AppError::validation(
                "date_of_birth",
                "must be in the past",
            ));
        }
        let age = Utc::now().year() - dob.year();
        if age < 18 {
            return Err(AppError::validation(
                "date_of_birth",
                "customer must be at least 18 years old",
            ));
        }
        if age > 120 {
            return Err(AppError::validation("date_of_birth", "invalid"));
        }
        Ok(())
    }
}

// ── WishlistItem ────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct WishlistItem {
    pub id: WishlistItemId,
    pub customer_id: CustomerId,
    pub product_id: Uuid,
    pub product_name: String,
    pub base_price: f64,
    pub added_at: DateTime<Utc>,
    #[doc(hidden)]
    pub domain_events: Vec<Box<dyn DomainEvent>>,
}

impl WishlistItem {
    pub fn create(
        customer_id: CustomerId,
        product_id: Uuid,
        product_name: String,
        base_price: f64,
    ) -> AppResult<Self> {
        if product_name.trim().is_empty() {
            return Err(AppError::validation("product_name", "must not be empty"));
        }
        if base_price.is_nan() || base_price < 0.0 {
            return Err(AppError::validation("base_price", "must be >= 0"));
        }
        let now = Utc::now();
        let mut item = Self {
            id: WishlistItemId::new(),
            customer_id,
            product_id,
            product_name: product_name.clone(),
            base_price,
            added_at: now,
            domain_events: Vec::new(),
        };
        item.domain_events.push(Box::new(WishlistItemAdded {
            customer_id,
            product_id,
            product_name,
            occurred_at: now,
        }));
        Ok(item)
    }

    pub fn emit_removed(&mut self) {
        self.domain_events.push(Box::new(WishlistItemRemoved {
            customer_id: self.customer_id,
            product_id: self.product_id,
            occurred_at: Utc::now(),
        }));
    }

    pub fn drain_events(&mut self) -> Vec<Box<dyn DomainEvent>> {
        std::mem::take(&mut self.domain_events)
    }
}
