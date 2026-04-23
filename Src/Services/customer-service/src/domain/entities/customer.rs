use chrono::{DateTime, Utc};
use uuid::Uuid;

use ddd_domain::{define_aggregate, define_entity, impl_aggregate, impl_aggregate_events};
use ddd_shared_kernel::{AppError, AppResult, DomainEvent};

use crate::domain::enums::{AddressType, ContactNumberType};
use crate::domain::events::{
    CustomerCreated, CustomerEmailUpdated, LoyaltyPointsUpdated, WishlistItemAdded,
    WishlistItemRemoved,
};
use crate::domain::ids::{ContactNumberId, CustomerAddressId, CustomerId, WishlistItemId};

// ── CustomerContactNumber ─────────────────────────────────────────────────────

define_entity!(CustomerContactNumber, ContactNumberId, {
    pub customer_id:  CustomerId,
    pub contact_type: ContactNumberType,
    pub phone_number: String,
    pub is_primary:   bool,
    pub verified:     bool,
    pub created_at:   DateTime<Utc>,
    pub updated_at:   Option<DateTime<Utc>>,
});

impl CustomerContactNumber {
    pub fn new(
        id:           ContactNumberId,
        customer_id:  CustomerId,
        contact_type: ContactNumberType,
        phone_number: String,
        is_primary:   bool,
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

// ── CustomerAddress ───────────────────────────────────────────────────────────

define_entity!(CustomerAddress, CustomerAddressId, {
    pub customer_id:  CustomerId,
    pub label:        String,
    pub address_type: AddressType,
    pub line1:        String,
    pub line2:        Option<String>,
    pub city:         String,
    pub state:        Option<String>,
    pub postal_code:  String,
    pub country_code: String,
    pub is_primary:   bool,
    pub created_at:   DateTime<Utc>,
    pub updated_at:   Option<DateTime<Utc>>,
});

impl CustomerAddress {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id:           CustomerAddressId,
        customer_id:  CustomerId,
        label:        String,
        address_type: AddressType,
        line1:        String,
        line2:        Option<String>,
        city:         String,
        state:        Option<String>,
        postal_code:  String,
        country_code: String,
        is_primary:   bool,
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

    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        label:        String,
        line1:        String,
        line2:        Option<String>,
        city:         String,
        state:        Option<String>,
        postal_code:  String,
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
            return Err(AppError::validation("postal_code", "must be 1-20 characters"));
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

// ── Customer ──────────────────────────────────────────────────────────────────

define_aggregate!(Customer, CustomerId, {
    pub user_id:                   Uuid,
    pub first_name:                String,
    pub last_name:                 String,
    pub email:                     Option<String>,
    pub membership_number:         String,
    pub join_date:                 DateTime<Utc>,
    pub expiry_date:               DateTime<Utc>,
    pub country_code:              String,
    pub loyalty_points:            i32,
    pub preferred_contact_method:  Option<i32>,
    pub preferred_address_type:    Option<i32>,
    pub created_by:                Option<String>,
    pub updated_by:                Option<String>,
    pub contact_numbers:           Vec<CustomerContactNumber>,
    pub addresses:                 Vec<CustomerAddress>,
});

impl_aggregate!(Customer, CustomerId);
impl_aggregate_events!(Customer);

impl Customer {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        id:               CustomerId,
        user_id:          Uuid,
        first_name:       String,
        last_name:        String,
        email:            Option<String>,
        membership_number: String,
        join_date:        DateTime<Utc>,
        expiry_date:      DateTime<Utc>,
        country_code:     String,
    ) -> AppResult<Self> {
        if user_id.is_nil() {
            return Err(AppError::validation("user_id", "must not be nil"));
        }
        let now = Utc::now();
        let mut c = Self {
            id,
            version: 0,
            domain_events: Vec::new(),
            created_at: now,
            updated_at: now,
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
            created_by: None,
            updated_by: None,
            contact_numbers: Vec::new(),
            addresses: Vec::new(),
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
            self.updated_at = Utc::now();
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
        self.updated_at = Utc::now();
        self.validate_personal_info()
    }

    pub fn add_loyalty_points(&mut self, points: i32, reason: String) -> AppResult<()> {
        if points <= 0 {
            return Err(AppError::validation("points", "must be positive"));
        }
        let previous = self.loyalty_points;
        self.loyalty_points += points;
        self.updated_at = Utc::now();
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
        self.updated_at = Utc::now();
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
        address_id:   CustomerAddressId,
        label:        String,
        line1:        String,
        line2:        Option<String>,
        city:         String,
        state:        Option<String>,
        postal_code:  String,
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
            return Err(AppError::not_found("CustomerAddress", address_id.to_string()));
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
            return Err(AppError::validation("expiry_date", "must be after join_date"));
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
            return Err(AppError::validation("first_name", "must be 1-100 characters"));
        }
        if self.last_name.len() > 100 {
            return Err(AppError::validation("last_name", "must be at most 100 characters"));
        }
        Ok(())
    }
}

// ── WishlistItem ──────────────────────────────────────────────────────────────

define_aggregate!(WishlistItem, WishlistItemId, {
    pub customer_id:  CustomerId,
    pub product_id:   Uuid,
    pub product_name: String,
    pub base_price:   f64,
});

impl_aggregate!(WishlistItem, WishlistItemId);
impl_aggregate_events!(WishlistItem);

impl WishlistItem {
    pub fn create(
        customer_id:  CustomerId,
        product_id:   Uuid,
        product_name: String,
        base_price:   f64,
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
            version: 0,
            domain_events: Vec::new(),
            created_at: now,
            updated_at: now,
            customer_id,
            product_id,
            product_name: product_name.clone(),
            base_price,
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
