use chrono::{DateTime, Utc};
use ddd_application::impl_command;
use uuid::Uuid;

use crate::domain::entities::{Brand, Product, ProductCategory, TaxConfiguration};
use crate::domain::ids::{BrandId, CategoryId, ProductId, ProductImageId, ProductVariantId, TaxConfigId};

// ── Product ───────────────────────────────────────────────────────────────────

impl_command! {
    CreateProduct {
        sku:         String,
        name:        String,
        category_id: i32,
        base_price:  f64,
        cost_price:  f64,
        description: Option<String>,
        is_taxable:  bool,
    } -> Product
}

impl_command! {
    UpdateProduct {
        id:          ProductId,
        name:        String,
        category_id: i32,
        base_price:  f64,
        cost_price:  f64,
        is_taxable:  bool,
        description: Option<String>,
    } -> Product
}

impl_command! { DiscontinueProduct  { id: ProductId } -> Product }
impl_command! { ReactivateProduct   { id: ProductId } -> Product }

impl_command! {
    UpdateProductPricing { id: ProductId, price: f64 } -> Product
}

impl_command! {
    AssignProductBrand { id: ProductId, brand_id: Option<Uuid> } -> Product
}

impl_command! {
    SetProductDimensions {
        id:          ProductId,
        weight_grams: Option<i32>,
        width_cm:    Option<i32>,
        height_cm:   Option<i32>,
        depth_cm:    Option<i32>,
    } -> Product
}

impl_command! {
    SetProductSpecifications {
        id:    ProductId,
        specs: serde_json::Value,
    } -> Product
}

impl_command! {
    SetProductTags { id: ProductId, tags: Vec<String> } -> Product
}

impl_command! {
    SetProductTaxConfigurations {
        id:             ProductId,
        tax_config_ids: Vec<String>,
    } -> Product
}

impl_command! {
    AddProductVariant {
        product_id:          ProductId,
        sku:                 String,
        attributes_json:     String,
        price_override:      Option<f64>,
        description:         Option<String>,
        cost_price_override: Option<f64>,
        barcode:             Option<String>,
        barcode_type:        Option<String>,
        weight_grams:        Option<i32>,
        width_cm:            Option<i32>,
        height_cm:           Option<i32>,
        depth_cm:            Option<i32>,
    } -> Product
}

impl_command! {
    UpdateProductVariant {
        product_id:          ProductId,
        variant_id:          ProductVariantId,
        sku:                 String,
        attributes_json:     String,
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
    } -> Product
}

impl_command! {
    RemoveProductVariant { product_id: ProductId, variant_id: ProductVariantId } -> Product
}

impl_command! {
    SetDefaultVariant { product_id: ProductId, variant_id: ProductVariantId } -> Product
}

// ── Product image (presigned workflow) ────────────────────────────────────────

impl_command! {
    RequestProductImageUploadUrl {
        product_id:   ProductId,
        file_name:    String,
        content_type: String,
        is_main:      bool,
        sort_order:   i32,
        alt_text:     Option<String>,
    } -> (String, String, String)  // (upload_url, object_name, expires_at)
}

impl_command! {
    ConfirmProductImageUpload {
        product_id:   ProductId,
        object_name:  String,
        file_name:    String,
        content_type: String,
        is_main:      bool,
        sort_order:   i32,
        alt_text:     Option<String>,
    } -> Product
}

impl_command! {
    DeleteProductImage { product_id: ProductId, image_id: ProductImageId } -> Product
}

// ── Category ──────────────────────────────────────────────────────────────────

impl_command! {
    CreateCategory {
        name:               String,
        description:        Option<String>,
        parent_category_id: Option<i32>,
    } -> ProductCategory
}

impl_command! {
    UpdateCategory {
        id:                 CategoryId,
        name:               String,
        description:        Option<String>,
        parent_category_id: Option<i32>,
    } -> ProductCategory
}

impl_command! {
    DeleteCategory { id: CategoryId } -> ()
}

// ── Category image (presigned workflow) ──────────────────────────────────────

impl_command! {
    RequestCategoryImageUploadUrl {
        category_id:  CategoryId,
        file_name:    String,
        content_type: String,
    } -> (String, String, String)  // (upload_url, object_name, expires_at)
}

impl_command! {
    ConfirmCategoryImageUpload {
        category_id: CategoryId,
        object_name: String,
        public_url:  String,
    } -> ProductCategory
}

// ── Brand ─────────────────────────────────────────────────────────────────────

impl_command! {
    CreateBrand {
        name:        String,
        description: Option<String>,
        website:     Option<String>,
    } -> Brand
}

impl_command! {
    UpdateBrand {
        id:          BrandId,
        name:        String,
        description: Option<String>,
        website:     Option<String>,
    } -> Brand
}

impl_command! { ActivateBrand   { id: BrandId } -> Brand }
impl_command! { DeactivateBrand { id: BrandId } -> Brand }

// ── Tax configuration ─────────────────────────────────────────────────────────

impl_command! {
    CreateTaxConfiguration {
        name:           String,
        code:           String,
        tax_type:       String,
        tax_rate:       f64,
        location_id:    i32,
        category_id:    Option<i32>,
        effective_date: DateTime<Utc>,
        expiry_date:    Option<DateTime<Utc>>,
    } -> TaxConfiguration
}

impl_command! {
    UpdateTaxConfiguration {
        id:             TaxConfigId,
        name:           String,
        code:           String,
        tax_type:       String,
        tax_rate:       f64,
        effective_date: DateTime<Utc>,
        expiry_date:    Option<DateTime<Utc>>,
    } -> TaxConfiguration
}

impl_command! { ActivateTaxConfiguration   { id: TaxConfigId } -> TaxConfiguration }
impl_command! { DeactivateTaxConfiguration { id: TaxConfigId } -> TaxConfiguration }
impl_command! { DeleteTaxConfiguration     { id: TaxConfigId } -> () }
