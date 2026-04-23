use chrono::{DateTime, Utc};
use ddd_application::impl_command;
use uuid::Uuid;

use crate::domain::entities::{Brand, Product, ProductCategory, TaxConfiguration};
use crate::domain::ids::{BrandId, CategoryId, ProductId, ProductImageId, ProductVariantId, TaxConfigId};

// ── Product ───────────────────────────────────────────────────────────────────

pub struct CreateProduct {
    pub sku:         String,
    pub name:        String,
    pub category_id: i32,
    pub base_price:  f64,
    pub cost_price:  f64,
    pub description: Option<String>,
    pub is_taxable:  bool,
}
impl_command!(CreateProduct, Product);

pub struct UpdateProduct {
    pub id:          ProductId,
    pub name:        String,
    pub category_id: i32,
    pub base_price:  f64,
    pub cost_price:  f64,
    pub is_taxable:  bool,
    pub description: Option<String>,
}
impl_command!(UpdateProduct, Product);

pub struct DiscontinueProduct {
    pub id: ProductId,
}
impl_command!(DiscontinueProduct, Product);

pub struct ReactivateProduct {
    pub id: ProductId,
}
impl_command!(ReactivateProduct, Product);

pub struct UpdateProductPricing {
    pub id:    ProductId,
    pub price: f64,
}
impl_command!(UpdateProductPricing, Product);

pub struct AssignProductBrand {
    pub id:       ProductId,
    pub brand_id: Option<Uuid>,
}
impl_command!(AssignProductBrand, Product);

pub struct SetProductDimensions {
    pub id:           ProductId,
    pub weight_grams: Option<i32>,
    pub width_cm:     Option<i32>,
    pub height_cm:    Option<i32>,
    pub depth_cm:     Option<i32>,
}
impl_command!(SetProductDimensions, Product);

pub struct SetProductSpecifications {
    pub id:    ProductId,
    pub specs: serde_json::Value,
}
impl_command!(SetProductSpecifications, Product);

pub struct SetProductTags {
    pub id:   ProductId,
    pub tags: Vec<String>,
}
impl_command!(SetProductTags, Product);

pub struct SetProductTaxConfigurations {
    pub id:             ProductId,
    pub tax_config_ids: Vec<String>,
}
impl_command!(SetProductTaxConfigurations, Product);

pub struct AddProductVariant {
    pub product_id:          ProductId,
    pub sku:                 String,
    pub attributes_json:     String,
    pub price_override:      Option<f64>,
    pub description:         Option<String>,
    pub cost_price_override: Option<f64>,
    pub barcode:             Option<String>,
    pub barcode_type:        Option<String>,
    pub weight_grams:        Option<i32>,
    pub width_cm:            Option<i32>,
    pub height_cm:           Option<i32>,
    pub depth_cm:            Option<i32>,
}
impl_command!(AddProductVariant, Product);

pub struct UpdateProductVariant {
    pub product_id:          ProductId,
    pub variant_id:          ProductVariantId,
    pub sku:                 String,
    pub attributes_json:     String,
    pub price_override:      Option<f64>,
    pub description:         Option<String>,
    pub is_active:           bool,
    pub cost_price_override: Option<f64>,
    pub barcode:             Option<String>,
    pub barcode_type:        Option<String>,
    pub weight_grams:        Option<i32>,
    pub width_cm:            Option<i32>,
    pub height_cm:           Option<i32>,
    pub depth_cm:            Option<i32>,
}
impl_command!(UpdateProductVariant, Product);

pub struct RemoveProductVariant {
    pub product_id: ProductId,
    pub variant_id: ProductVariantId,
}
impl_command!(RemoveProductVariant, Product);

pub struct SetDefaultVariant {
    pub product_id: ProductId,
    pub variant_id: ProductVariantId,
}
impl_command!(SetDefaultVariant, Product);

pub struct RequestProductImageUploadUrl {
    pub product_id:   ProductId,
    pub file_name:    String,
    pub content_type: String,
    pub is_main:      bool,
    pub sort_order:   i32,
    pub alt_text:     Option<String>,
}
impl_command!(RequestProductImageUploadUrl, (String, String, String));

pub struct ConfirmProductImageUpload {
    pub product_id:   ProductId,
    pub object_name:  String,
    pub file_name:    String,
    pub content_type: String,
    pub is_main:      bool,
    pub sort_order:   i32,
    pub alt_text:     Option<String>,
}
impl_command!(ConfirmProductImageUpload, Product);

pub struct DeleteProductImage {
    pub product_id: ProductId,
    pub image_id:   ProductImageId,
}
impl_command!(DeleteProductImage, Product);

// ── Category ──────────────────────────────────────────────────────────────────

pub struct CreateCategory {
    pub name:               String,
    pub description:        Option<String>,
    pub parent_category_id: Option<i32>,
}
impl_command!(CreateCategory, ProductCategory);

pub struct UpdateCategory {
    pub id:                 CategoryId,
    pub name:               String,
    pub description:        Option<String>,
    pub parent_category_id: Option<i32>,
}
impl_command!(UpdateCategory, ProductCategory);

pub struct DeleteCategory {
    pub id: CategoryId,
}
impl_command!(DeleteCategory, ());

pub struct RequestCategoryImageUploadUrl {
    pub category_id:  CategoryId,
    pub file_name:    String,
    pub content_type: String,
}
impl_command!(RequestCategoryImageUploadUrl, (String, String, String));

pub struct ConfirmCategoryImageUpload {
    pub category_id: CategoryId,
    pub object_name: String,
    pub public_url:  String,
}
impl_command!(ConfirmCategoryImageUpload, ProductCategory);

// ── Brand ─────────────────────────────────────────────────────────────────────

pub struct CreateBrand {
    pub name:        String,
    pub description: Option<String>,
    pub website:     Option<String>,
}
impl_command!(CreateBrand, Brand);

pub struct UpdateBrand {
    pub id:          BrandId,
    pub name:        String,
    pub description: Option<String>,
    pub website:     Option<String>,
}
impl_command!(UpdateBrand, Brand);

pub struct ActivateBrand {
    pub id: BrandId,
}
impl_command!(ActivateBrand, Brand);

pub struct DeactivateBrand {
    pub id: BrandId,
}
impl_command!(DeactivateBrand, Brand);

// ── Tax configuration ─────────────────────────────────────────────────────────

pub struct CreateTaxConfiguration {
    pub name:           String,
    pub code:           String,
    pub tax_type:       String,
    pub tax_rate:       f64,
    pub location_id:    i32,
    pub category_id:    Option<i32>,
    pub effective_date: DateTime<Utc>,
    pub expiry_date:    Option<DateTime<Utc>>,
}
impl_command!(CreateTaxConfiguration, TaxConfiguration);

pub struct UpdateTaxConfiguration {
    pub id:             TaxConfigId,
    pub name:           String,
    pub code:           String,
    pub tax_type:       String,
    pub tax_rate:       f64,
    pub effective_date: DateTime<Utc>,
    pub expiry_date:    Option<DateTime<Utc>>,
}
impl_command!(UpdateTaxConfiguration, TaxConfiguration);

pub struct ActivateTaxConfiguration {
    pub id: TaxConfigId,
}
impl_command!(ActivateTaxConfiguration, TaxConfiguration);

pub struct DeactivateTaxConfiguration {
    pub id: TaxConfigId,
}
impl_command!(DeactivateTaxConfiguration, TaxConfiguration);

pub struct DeleteTaxConfiguration {
    pub id: TaxConfigId,
}
impl_command!(DeleteTaxConfiguration, ());
