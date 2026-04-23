use uuid::Uuid;

use crate::domain::ids::{ProductId, ProductVariantId};

#[derive(Debug, Clone)]
pub struct ProductVariant {
    pub id:                  ProductVariantId,
    pub product_id:          ProductId,
    pub sku:                 String,
    pub description:         Option<String>,
    pub price_override:      Option<f64>,
    pub cost_price_override: Option<f64>,
    pub is_active:           bool,
    pub is_default:          bool,
    pub barcode:             Option<String>,
    pub barcode_type:        Option<String>,
    pub weight_grams:        Option<i32>,
    pub width_cm:            Option<i32>,
    pub height_cm:           Option<i32>,
    pub depth_cm:            Option<i32>,
    pub attributes:          serde_json::Value,
}

impl ProductVariant {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        product_id:          ProductId,
        sku:                 String,
        attributes_json:     &str,
        price_override:      Option<f64>,
        description:         Option<String>,
        cost_price_override: Option<f64>,
        barcode:             Option<String>,
        barcode_type:        Option<String>,
        weight_grams:        Option<i32>,
        width_cm:            Option<i32>,
        height_cm:           Option<i32>,
        depth_cm:            Option<i32>,
    ) -> Self {
        let attributes = serde_json::from_str(attributes_json)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        Self {
            id: ProductVariantId::from_uuid(Uuid::new_v4()),
            product_id,
            sku,
            description,
            price_override,
            cost_price_override,
            is_active: true,
            is_default: false,
            barcode,
            barcode_type,
            weight_grams,
            width_cm,
            height_cm,
            depth_cm,
            attributes,
        }
    }
}
