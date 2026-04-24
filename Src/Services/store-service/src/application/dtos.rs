use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::entities::{Register, Store, StoreSchedule};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressDto {
    pub street:      String,
    pub city:        String,
    pub postal_code: String,
    pub country:     String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeolocationDto {
    pub latitude:  f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreScheduleDto {
    pub day_of_week: u8,
    pub open_time:   Option<String>,
    pub close_time:  Option<String>,
    pub is_closed:   bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreDto {
    pub id:             i32,
    pub name:           String,
    pub location_id:    i32,
    pub address:        AddressDto,
    pub phone:          String,
    pub geolocation:    GeolocationDto,
    pub schedules:      Vec<StoreScheduleDto>,
    pub services:       Vec<String>,
    pub status:         String,
    pub is_operational: bool,
    pub created_at:     DateTime<Utc>,
    pub updated_at:     DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedStoreDto {
    pub items:     Vec<StoreDto>,
    pub total:     i64,
    pub page:      i32,
    pub page_size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterDto {
    pub id:              i32,
    pub store_id:        i32,
    pub name:            String,
    pub current_balance: Decimal,
    pub status:          String,
    pub is_open:         bool,
    pub last_open:       Option<DateTime<Utc>>,
    pub last_close:      Option<DateTime<Utc>>,
    pub created_at:      DateTime<Utc>,
    pub updated_at:      DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedRegisterDto {
    pub items:     Vec<RegisterDto>,
    pub total:     i64,
    pub page:      i32,
    pub page_size: i32,
}

pub fn map_schedule(s: &StoreSchedule) -> StoreScheduleDto {
    StoreScheduleDto {
        day_of_week: s.day_of_week,
        open_time:   s.open_time.clone(),
        close_time:  s.close_time.clone(),
        is_closed:   s.is_closed,
    }
}

pub fn map_store(s: &Store) -> StoreDto {
    StoreDto {
        id:          s.id.0,
        name:        s.name.clone(),
        location_id: s.location_id,
        address: AddressDto {
            street:      s.address_street.clone(),
            city:        s.address_city.clone(),
            postal_code: s.address_postal_code.clone(),
            country:     s.address_country.clone(),
        },
        phone: s.phone.clone(),
        geolocation: GeolocationDto {
            latitude:  s.geo_latitude,
            longitude: s.geo_longitude,
        },
        schedules:      s.schedules.iter().map(map_schedule).collect(),
        services:       s.services.clone(),
        status:         s.status.as_str().to_string(),
        is_operational: s.is_operational(),
        created_at:     s.created_at,
        updated_at:     s.updated_at,
    }
}

pub fn map_register(r: &Register) -> RegisterDto {
    RegisterDto {
        id:              r.id.0,
        store_id:        r.store_id.0,
        name:            r.name.clone(),
        current_balance: r.current_balance,
        status:          r.status.as_str().to_string(),
        is_open:         r.is_open(),
        last_open:       r.last_open,
        last_close:      r.last_close,
        created_at:      r.created_at,
        updated_at:      r.updated_at,
    }
}
