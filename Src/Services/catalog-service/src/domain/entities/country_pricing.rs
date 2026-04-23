use chrono::{DateTime, Utc};
use ddd_domain::define_entity;
use uuid::Uuid;

use crate::domain::ids::{PricingId, ProductId};

define_entity!(CountryPricing, PricingId, {
    pub product_id:     ProductId,
    pub country_code:   String,
    pub price:          f64,
    pub effective_date: DateTime<Utc>,
});

impl CountryPricing {
    pub fn create(
        product_id:     ProductId,
        country_code:   String,
        price:          f64,
        effective_date: DateTime<Utc>,
    ) -> Self {
        Self {
            id: PricingId::from_uuid(Uuid::new_v4()),
            product_id,
            country_code,
            price,
            effective_date,
        }
    }
}
