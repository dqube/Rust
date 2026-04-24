use chrono::Utc;
use serde::{Deserialize, Serialize};

use ddd_domain::{define_aggregate, impl_aggregate, impl_aggregate_events};
use ddd_shared_kernel::{AppError, AppResult, DomainEvent};

use crate::domain::enums::StoreStatus;
use crate::domain::events::{
    StoreCreated, StoreInformationUpdated, StoreLogoUpdated, StoreStatusChanged,
};
use crate::domain::ids::StoreId;

// ── StoreSchedule (value object) ────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreSchedule {
    pub day_of_week: u8,
    pub open_time:   Option<String>,
    pub close_time:  Option<String>,
    pub is_closed:   bool,
}

impl StoreSchedule {
    pub fn new(
        day_of_week: u8,
        open_time:   Option<String>,
        close_time:  Option<String>,
        is_closed:   bool,
    ) -> AppResult<Self> {
        if day_of_week > 6 {
            return Err(AppError::validation("day_of_week", "must be 0-6"));
        }
        Ok(Self { day_of_week, open_time, close_time, is_closed })
    }
}

// ── Store (aggregate) ───────────────────────────────────────────────────────

define_aggregate!(Store, StoreId, {
    pub name:                String,
    pub location_id:         i32,
    pub address_street:      String,
    pub address_city:        String,
    pub address_postal_code: String,
    pub address_country:     String,
    pub phone:               String,
    pub geo_latitude:        f64,
    pub geo_longitude:       f64,
    pub schedules:           Vec<StoreSchedule>,
    pub services:            Vec<String>,
    pub status:              StoreStatus,
    pub logo_object_name:    Option<String>,
});

impl_aggregate!(Store, StoreId);
impl_aggregate_events!(Store);

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        name:                String,
        location_id:         i32,
        address_street:      String,
        address_city:        String,
        address_postal_code: String,
        address_country:     String,
        phone:               String,
        geo_latitude:        f64,
        geo_longitude:       f64,
        schedules:           Vec<StoreSchedule>,
        services:            Vec<String>,
    ) -> AppResult<Self> {
        Self::validate_fields(
            &name, &address_street, &address_city, &address_country, &phone,
            geo_latitude, geo_longitude,
        )?;
        let now = Utc::now();
        Ok(Self {
            id: StoreId(0),
            version: 0,
            created_at: now,
            updated_at: now,
            domain_events: Vec::new(),
            name,
            location_id,
            address_street,
            address_city,
            address_postal_code,
            address_country,
            phone,
            geo_latitude,
            geo_longitude,
            schedules,
            services,
            status: StoreStatus::Active,
            logo_object_name: None,
        })
    }

    /// Emit `StoreCreated` after the repository has assigned the real id.
    pub fn emit_created(&mut self) {
        self.record_event(StoreCreated {
            store_id:    self.id,
            name:        self.name.clone(),
            city:        self.address_city.clone(),
            occurred_at: Utc::now(),
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_information(
        &mut self,
        name:                String,
        address_street:      String,
        address_city:        String,
        address_postal_code: String,
        address_country:     String,
        phone:               String,
        geo_latitude:        f64,
        geo_longitude:       f64,
        schedules:           Vec<StoreSchedule>,
        services:            Vec<String>,
    ) -> AppResult<()> {
        Self::validate_fields(
            &name, &address_street, &address_city, &address_country, &phone,
            geo_latitude, geo_longitude,
        )?;
        self.name                = name;
        self.address_street      = address_street;
        self.address_city        = address_city;
        self.address_postal_code = address_postal_code;
        self.address_country     = address_country;
        self.phone               = phone;
        self.geo_latitude        = geo_latitude;
        self.geo_longitude       = geo_longitude;
        self.schedules           = schedules;
        self.services            = services;
        self.updated_at          = Utc::now();
        self.record_event(StoreInformationUpdated {
            store_id:    self.id,
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    pub fn change_status(&mut self, status: StoreStatus) {
        if self.status == status {
            return;
        }
        let old = self.status.clone();
        self.status     = status.clone();
        self.updated_at = Utc::now();
        self.record_event(StoreStatusChanged {
            store_id:    self.id,
            old_status:  old,
            new_status:  status,
            occurred_at: Utc::now(),
        });
    }

    pub fn set_logo(&mut self, object_name: String) {
        self.logo_object_name = Some(object_name.clone());
        self.updated_at       = Utc::now();
        self.record_event(StoreLogoUpdated {
            store_id:    self.id,
            object_name: Some(object_name),
            occurred_at: Utc::now(),
        });
    }

    pub fn remove_logo(&mut self) {
        self.logo_object_name = None;
        self.updated_at       = Utc::now();
        self.record_event(StoreLogoUpdated {
            store_id:    self.id,
            object_name: None,
            occurred_at: Utc::now(),
        });
    }

    pub fn is_operational(&self) -> bool {
        self.status == StoreStatus::Active
    }

    pub fn drain_events(&mut self) -> Vec<Box<dyn DomainEvent>> {
        std::mem::take(&mut self.domain_events)
    }

    fn validate_fields(
        name:    &str,
        street:  &str,
        city:    &str,
        country: &str,
        phone:   &str,
        lat:     f64,
        lon:     f64,
    ) -> AppResult<()> {
        if name.trim().is_empty() || name.len() > 100 {
            return Err(AppError::validation("name", "must be 1-100 characters"));
        }
        if street.trim().is_empty() {
            return Err(AppError::validation("address_street", "must not be empty"));
        }
        if city.trim().is_empty() {
            return Err(AppError::validation("address_city", "must not be empty"));
        }
        if country.trim().is_empty() {
            return Err(AppError::validation("address_country", "must not be empty"));
        }
        if phone.trim().is_empty() {
            return Err(AppError::validation("phone", "must not be empty"));
        }
        if !(-90.0..=90.0).contains(&lat) {
            return Err(AppError::validation("geo_latitude", "must be between -90 and 90"));
        }
        if !(-180.0..=180.0).contains(&lon) {
            return Err(AppError::validation("geo_longitude", "must be between -180 and 180"));
        }
        Ok(())
    }
}
