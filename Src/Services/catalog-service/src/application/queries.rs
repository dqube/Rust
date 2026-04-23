use ddd_application::impl_query;
use ddd_shared_kernel::{Page, PageRequest};

use crate::domain::entities::{Brand, Product, ProductCategory, TaxConfiguration};
use crate::domain::ids::{BrandId, CategoryId, ProductId, TaxConfigId};

// ── Product ───────────────────────────────────────────────────────────────────

impl_query! { GetProduct { id: ProductId } -> Option<Product> }

impl_query! {
    ListProducts {
        search:       Option<String>,
        category_id:  Option<i32>,
        min_price:    Option<f64>,
        max_price:    Option<f64>,
        sort_by:      Option<String>,
        sort_desc:    bool,
        req:          PageRequest,
    } -> Page<Product>
}

// ── Category ──────────────────────────────────────────────────────────────────

impl_query! { GetCategory    { id: CategoryId }                      -> Option<ProductCategory> }
impl_query! {
    ListCategories {
        parent_id: Option<i32>,
    } -> Vec<ProductCategory>
}

// ── Brand ─────────────────────────────────────────────────────────────────────

impl_query! { GetBrand { id: BrandId } -> Option<Brand> }

impl_query! {
    ListBrands {
        search:      Option<String>,
        active_only: bool,
        req:         PageRequest,
    } -> Page<Brand>
}

// ── Tax configuration ─────────────────────────────────────────────────────────

impl_query! { GetTaxConfig { id: TaxConfigId } -> Option<TaxConfiguration> }

impl_query! {
    ListTaxConfigs {
        location_id: Option<i32>,
        tax_type:    Option<String>,
        active_only: bool,
    } -> Vec<TaxConfiguration>
}

impl_query! {
    GetApplicableTaxConfigs {
        location_id: i32,
        category_id: Option<i32>,
    } -> Vec<TaxConfiguration>
}

impl_query! {
    CalculateTax {
        location_id: i32,
        category_id: Option<i32>,
        amount:      f64,
    } -> (f64, f64, Vec<(String, String, String, f64, f64)>)
    // (tax_amount, total_amount, [(name, code, tax_type, rate, amount)])
}
