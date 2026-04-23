use ddd_application::impl_query;
use ddd_shared_kernel::{Page, PageRequest};

use crate::domain::entities::{Brand, Product, ProductCategory, TaxConfiguration};
use crate::domain::ids::{BrandId, CategoryId, ProductId, TaxConfigId};

// ── Product ───────────────────────────────────────────────────────────────────

pub struct GetProduct {
    pub id: ProductId,
}
impl_query!(GetProduct, Option<Product>);

pub struct ListProducts {
    pub search:      Option<String>,
    pub category_id: Option<i32>,
    pub min_price:   Option<f64>,
    pub max_price:   Option<f64>,
    pub sort_by:     Option<String>,
    pub sort_desc:   bool,
    pub req:         PageRequest,
}
impl_query!(ListProducts, Page<Product>);

// ── Category ──────────────────────────────────────────────────────────────────

pub struct GetCategory {
    pub id: CategoryId,
}
impl_query!(GetCategory, Option<ProductCategory>);

pub struct ListCategories {
    pub parent_id: Option<i32>,
}
impl_query!(ListCategories, Vec<ProductCategory>);

// ── Brand ─────────────────────────────────────────────────────────────────────

pub struct GetBrand {
    pub id: BrandId,
}
impl_query!(GetBrand, Option<Brand>);

pub struct ListBrands {
    pub search:      Option<String>,
    pub active_only: bool,
    pub req:         PageRequest,
}
impl_query!(ListBrands, Page<Brand>);

// ── Tax configuration ─────────────────────────────────────────────────────────

pub struct GetTaxConfig {
    pub id: TaxConfigId,
}
impl_query!(GetTaxConfig, Option<TaxConfiguration>);

pub struct ListTaxConfigs {
    pub location_id: Option<i32>,
    pub tax_type:    Option<String>,
    pub active_only: bool,
}
impl_query!(ListTaxConfigs, Vec<TaxConfiguration>);

pub struct GetApplicableTaxConfigs {
    pub location_id: i32,
    pub category_id: Option<i32>,
}
impl_query!(GetApplicableTaxConfigs, Vec<TaxConfiguration>);

pub struct CalculateTax {
    pub location_id: i32,
    pub category_id: Option<i32>,
    pub amount:      f64,
}
impl_query!(CalculateTax, (f64, f64, Vec<(String, String, String, f64, f64)>));
