use ddd_domain::define_entity;
use uuid::Uuid;

use crate::domain::ids::{ProductId, ProductImageId};

define_entity!(ProductImage, ProductImageId, {
    pub product_id: ProductId,
    pub url:        String,
    pub is_main:    bool,
    pub sort_order: i32,
    pub alt_text:   Option<String>,
});

impl ProductImage {
    pub fn create(
        product_id: ProductId,
        url:        String,
        is_main:    bool,
        sort_order: i32,
        alt_text:   Option<String>,
    ) -> Self {
        Self {
            id: ProductImageId::from_uuid(Uuid::new_v4()),
            product_id,
            url,
            is_main,
            sort_order,
            alt_text,
        }
    }
}
