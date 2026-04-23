use chrono::{DateTime, Utc};
use uuid::Uuid;

use ddd_shared_kernel::AppError;

use crate::domain::ids::{ProductId, ProductImageId, ProductVariantId};

use super::country_pricing::CountryPricing;
use super::product_image::ProductImage;
use super::product_variant::ProductVariant;

#[derive(Debug, Clone)]
pub struct Product {
    pub id:                           ProductId,
    pub sku:                          String,
    pub name:                         String,
    pub description:                  Option<String>,
    pub slug:                         Option<String>,
    pub category_id:                  i32,
    pub weight_grams:                 Option<i32>,
    pub width_cm:                     Option<i32>,
    pub height_cm:                    Option<i32>,
    pub depth_cm:                     Option<i32>,
    pub brand_id:                     Option<Uuid>,
    pub base_price:                   f64,
    pub cost_price:                   f64,
    pub is_taxable:                   bool,
    pub is_discontinued:              bool,
    pub discontinued_at:              Option<DateTime<Utc>>,
    pub is_inventory_tracked:         bool,
    pub specifications:               serde_json::Value,
    pub tags:                         Vec<String>,
    pub assigned_tax_config_ids:      Vec<String>,
    pub average_rating:               Option<f64>,
    pub total_reviews:                i32,
    pub active_promotion_id:          Option<Uuid>,
    pub active_promotion_name:        Option<String>,
    pub promotion_discount_percentage: Option<f64>,
    pub promotion_valid_until:        Option<DateTime<Utc>>,
    pub created_at:                   DateTime<Utc>,
    pub created_by:                   Option<String>,
    pub updated_at:                   Option<DateTime<Utc>>,
    pub updated_by:                   Option<String>,
    // Children (loaded eagerly)
    pub variants:        Vec<ProductVariant>,
    pub images:          Vec<ProductImage>,
    pub country_pricing: Vec<CountryPricing>,
}

impl Product {
    pub fn create(
        sku:         String,
        name:        String,
        category_id: i32,
        base_price:  f64,
        cost_price:  f64,
        description: Option<String>,
        is_taxable:  bool,
    ) -> Result<Self, AppError> {
        if sku.trim().is_empty() {
            return Err(AppError::validation("sku", "SKU cannot be empty"));
        }
        if name.trim().is_empty() {
            return Err(AppError::validation("name", "Product name cannot be empty"));
        }
        if base_price < 0.0 {
            return Err(AppError::validation("base_price", "Base price cannot be negative"));
        }
        let slug = slugify(&name);
        Ok(Self {
            id: ProductId::from_uuid(Uuid::new_v4()),
            sku,
            name,
            description,
            slug: Some(slug),
            category_id,
            weight_grams: None,
            width_cm: None,
            height_cm: None,
            depth_cm: None,
            brand_id: None,
            base_price,
            cost_price,
            is_taxable,
            is_discontinued: false,
            discontinued_at: None,
            is_inventory_tracked: true,
            specifications: serde_json::Value::Object(serde_json::Map::new()),
            tags: Vec::new(),
            assigned_tax_config_ids: Vec::new(),
            average_rating: None,
            total_reviews: 0,
            active_promotion_id: None,
            active_promotion_name: None,
            promotion_discount_percentage: None,
            promotion_valid_until: None,
            created_at: Utc::now(),
            created_by: None,
            updated_at: None,
            updated_by: None,
            variants: Vec::new(),
            images: Vec::new(),
            country_pricing: Vec::new(),
        })
    }

    pub fn update_basic_info(
        &mut self,
        name:        String,
        category_id: i32,
        base_price:  f64,
        cost_price:  f64,
        is_taxable:  bool,
        description: Option<String>,
    ) -> Result<(), AppError> {
        if name.trim().is_empty() {
            return Err(AppError::validation("name", "Product name cannot be empty"));
        }
        self.name = name;
        self.category_id = category_id;
        self.base_price = base_price;
        self.cost_price = cost_price;
        self.is_taxable = is_taxable;
        self.description = description;
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn update_pricing(&mut self, price: f64) {
        self.base_price = price;
        self.updated_at = Some(Utc::now());
    }

    pub fn discontinue(&mut self) -> Result<(), AppError> {
        if self.is_discontinued {
            return Err(AppError::conflict("Product is already discontinued"));
        }
        self.is_discontinued = true;
        self.discontinued_at = Some(Utc::now());
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn reactivate(&mut self) -> Result<(), AppError> {
        if !self.is_discontinued {
            return Err(AppError::conflict("Product is not discontinued"));
        }
        self.is_discontinued = false;
        self.discontinued_at = None;
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn assign_brand(&mut self, brand_id: Option<Uuid>) {
        self.brand_id = brand_id;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_dimensions(
        &mut self,
        weight_grams: Option<i32>,
        width_cm:     Option<i32>,
        height_cm:    Option<i32>,
        depth_cm:     Option<i32>,
    ) {
        self.weight_grams = weight_grams;
        self.width_cm = width_cm;
        self.height_cm = height_cm;
        self.depth_cm = depth_cm;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_specifications(&mut self, specs: serde_json::Value) {
        self.specifications = specs;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_tags(&mut self, tags: Vec<String>) {
        self.tags = tags;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_tax_configurations(&mut self, ids: Vec<String>) {
        self.assigned_tax_config_ids = ids;
        self.updated_at = Some(Utc::now());
    }

    pub fn add_image(&mut self, image: ProductImage) {
        if image.is_main {
            for img in &mut self.images {
                img.is_main = false;
            }
        }
        self.images.push(image);
        self.updated_at = Some(Utc::now());
    }

    pub fn remove_image(&mut self, image_id: ProductImageId) -> Result<(), AppError> {
        let pos = self.images.iter().position(|i| i.id == image_id)
            .ok_or_else(|| AppError::not_found("image", image_id.to_string()))?;
        self.images.remove(pos);
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn add_variant(&mut self, variant: ProductVariant) {
        self.variants.push(variant);
        self.updated_at = Some(Utc::now());
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_variant(
        &mut self,
        variant_id:          ProductVariantId,
        sku:                 String,
        attributes_json:     &str,
        price_override:      Option<f64>,
        description:         Option<String>,
        is_active:           bool,
        cost_price_override: Option<f64>,
        barcode:             Option<String>,
        barcode_type:        Option<String>,
        weight_grams:        Option<i32>,
        width_cm:            Option<i32>,
        height_cm:           Option<i32>,
        depth_cm:            Option<i32>,
    ) -> Result<(), AppError> {
        let v = self.variants.iter_mut()
            .find(|v| v.id == variant_id)
            .ok_or_else(|| AppError::not_found("variant", variant_id.to_string()))?;
        v.sku = sku;
        v.attributes = serde_json::from_str(attributes_json)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        v.price_override = price_override;
        v.description = description;
        v.is_active = is_active;
        v.cost_price_override = cost_price_override;
        v.barcode = barcode;
        v.barcode_type = barcode_type;
        v.weight_grams = weight_grams;
        v.width_cm = width_cm;
        v.height_cm = height_cm;
        v.depth_cm = depth_cm;
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn remove_variant(&mut self, variant_id: ProductVariantId) -> Result<(), AppError> {
        let pos = self.variants.iter().position(|v| v.id == variant_id)
            .ok_or_else(|| AppError::not_found("variant", variant_id.to_string()))?;
        self.variants.remove(pos);
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn set_default_variant(&mut self, variant_id: ProductVariantId) -> Result<(), AppError> {
        let exists = self.variants.iter().any(|v| v.id == variant_id);
        if !exists {
            return Err(AppError::not_found("variant", variant_id.to_string()));
        }
        for v in &mut self.variants {
            v.is_default = v.id == variant_id;
        }
        self.updated_at = Some(Utc::now());
        Ok(())
    }
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
