//! Catalog pass-through handlers — REST → gRPC proxy.

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use ddd_bff::prelude::*;

use crate::application::state::AppState;
use crate::proto_catalog::{
    ActivateBrandRequest, ActivateTaxConfigurationRequest, AddProductVariantRequest,
    AssignProductBrandRequest, BrandIdRequest, CalculateTaxRequest, ConfirmCategoryImageUploadRequest,
    ConfirmProductImageUploadRequest, CreateBrandRequest, CreateCategoryRequest,
    CreateProductRequest, CreateTaxConfigurationRequest, DeactivateBrandRequest,
    DeactivateTaxConfigurationRequest, DeleteCategoryRequest, DeleteProductImageRequest,
    DeleteTaxConfigurationRequest, GetApplicableTaxConfigurationsRequest, GetBrandRequest,
    GetCategoryRequest, GetProductRequest, GetTaxConfigurationRequest, ListBrandsRequest,
    ListCategoriesRequest, ListProductsRequest, ListTaxConfigurationsRequest,
    RemoveProductVariantRequest, RequestCategoryImageUploadUrlRequest,
    RequestProductImageUploadUrlRequest, SetDefaultVariantRequest, SetProductDimensionsRequest,
    SetProductSpecificationsRequest, SetProductTagsRequest, SetProductTaxConfigurationsRequest,
    TaxConfigIdRequest, UpdateBrandRequest, UpdateCategoryRequest, UpdateProductPricingRequest,
    UpdateProductRequest, UpdateProductVariantRequest, ProductIdRequest, UpdateTaxConfigurationRequest,
};

// ── Products ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListProductsParams {
    pub page:          Option<u32>,
    pub per_page:      Option<u32>,
    pub search:        Option<String>,
    pub category_id:   Option<i32>,
    pub min_price:     Option<f64>,
    pub max_price:     Option<f64>,
    pub sort_by:       Option<String>,
    pub sort_desc:     Option<bool>,
}

pub async fn list_products(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Query(params): Query<ListProductsParams>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            let has_category  = params.category_id.is_some();
            let has_price     = params.min_price.is_some() || params.max_price.is_some();
            state.catalog_client.client().list_products(ListProductsRequest {
                page:          params.page.unwrap_or(1),
                per_page:      params.per_page.unwrap_or(20),
                search:        params.search.unwrap_or_default(),
                category_id:   params.category_id.unwrap_or(0),
                has_category,
                min_price:     params.min_price.unwrap_or(0.0),
                max_price:     params.max_price.unwrap_or(0.0),
                has_price_range: has_price,
                sort_by:       params.sort_by.unwrap_or_default(),
                sort_desc:     params.sort_desc.unwrap_or(false),
            }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn create_product(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<CreateProductRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().create_product(req).await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn get_product(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().get_product(GetProductRequest { product_id: id }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn update_product(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<UpdateProductRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.product_id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().update_product(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn discontinue_product(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().discontinue_product(ProductIdRequest { product_id: id }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn reactivate_product(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().reactivate_product(ProductIdRequest { product_id: id }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn update_product_pricing(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<UpdateProductPricingRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.product_id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().update_product_pricing(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn assign_product_brand(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<AssignProductBrandRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.product_id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().assign_product_brand(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn set_product_dimensions(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<SetProductDimensionsRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.product_id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().set_product_dimensions(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn set_product_specifications(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<SetProductSpecificationsRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.product_id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().set_product_specifications(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn set_product_tags(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<SetProductTagsRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.product_id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().set_product_tags(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn set_product_tax_configurations(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<SetProductTaxConfigurationsRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.product_id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().set_product_tax_configurations(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn add_product_variant(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<AddProductVariantRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.product_id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().add_product_variant(req).await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn update_product_variant(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path((product_id, variant_id)): Path<(String, String)>,
    Json(mut req): Json<UpdateProductVariantRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.product_id = product_id;
    req.variant_id = variant_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().update_product_variant(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn remove_product_variant(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path((product_id, variant_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().remove_product_variant(RemoveProductVariantRequest {
                product_id,
                variant_id,
            }).await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn set_default_variant(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path((product_id, variant_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().set_default_variant(SetDefaultVariantRequest {
                product_id,
                variant_id,
            }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn request_product_image_upload_url(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<RequestProductImageUploadUrlRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.product_id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().request_product_image_upload_url(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn confirm_product_image_upload(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<ConfirmProductImageUploadRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.product_id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().confirm_product_image_upload(req).await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn delete_product_image(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path((product_id, image_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().delete_product_image(DeleteProductImageRequest {
                product_id,
                image_id,
            }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

// ── Categories ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListCategoriesParams {
    pub parent_category_id: Option<i32>,
    pub include_hierarchy:  Option<bool>,
}

pub async fn list_categories(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Query(params): Query<ListCategoriesParams>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            let has_parent = params.parent_category_id.is_some();
            state.catalog_client.client().list_categories(ListCategoriesRequest {
                parent_category_id: params.parent_category_id.unwrap_or(0),
                has_parent,
                include_hierarchy:  params.include_hierarchy.unwrap_or(false),
            }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn create_category(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<CreateCategoryRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().create_category(req).await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn get_category(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().get_category(GetCategoryRequest { category_id: id }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn update_category(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<i32>,
    Json(mut req): Json<UpdateCategoryRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.category_id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().update_category(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn delete_category(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().delete_category(DeleteCategoryRequest { category_id: id }).await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn request_category_image_upload_url(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<i32>,
    Json(mut req): Json<RequestCategoryImageUploadUrlRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.category_id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().request_category_image_upload_url(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn confirm_category_image_upload(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<i32>,
    Json(mut req): Json<ConfirmCategoryImageUploadRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.category_id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().confirm_category_image_upload(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

// ── Brands ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListBrandsParams {
    pub page:        Option<u32>,
    pub per_page:    Option<u32>,
    pub search:      Option<String>,
    pub active_only: Option<bool>,
}

pub async fn list_brands(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Query(params): Query<ListBrandsParams>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().list_brands(ListBrandsRequest {
                page:        params.page.unwrap_or(1),
                per_page:    params.per_page.unwrap_or(20),
                search:      params.search.unwrap_or_default(),
                active_only: params.active_only.unwrap_or(false),
            }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn create_brand(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<CreateBrandRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().create_brand(req).await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn get_brand(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().get_brand(GetBrandRequest { brand_id: id }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn update_brand(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<UpdateBrandRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.brand_id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().update_brand(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn activate_brand(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().activate_brand(BrandIdRequest { brand_id: id }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn deactivate_brand(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().deactivate_brand(BrandIdRequest { brand_id: id }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

// ── Tax Configurations ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListTaxConfigsParams {
    pub location_id:  Option<i32>,
    pub tax_type:     Option<String>,
    pub active_only:  Option<bool>,
}

pub async fn list_tax_configurations(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Query(params): Query<ListTaxConfigsParams>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            let has_location = params.location_id.is_some();
            let has_tax_type = params.tax_type.is_some();
            state.catalog_client.client().list_tax_configurations(ListTaxConfigurationsRequest {
                location_id:  params.location_id.unwrap_or(0),
                has_location,
                tax_type:     params.tax_type.unwrap_or_default(),
                has_tax_type,
                active_only:  params.active_only.unwrap_or(false),
            }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn create_tax_configuration(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<CreateTaxConfigurationRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().create_tax_configuration(req).await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn get_tax_configuration(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().get_tax_configuration(GetTaxConfigurationRequest { tax_config_id: id }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn update_tax_configuration(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<UpdateTaxConfigurationRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.tax_config_id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().update_tax_configuration(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn delete_tax_configuration(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().delete_tax_configuration(TaxConfigIdRequest { tax_config_id: id }).await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn activate_tax_configuration(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().activate_tax_configuration(ActivateTaxConfigurationRequest { tax_config_id: id }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn deactivate_tax_configuration(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.catalog_client.client().deactivate_tax_configuration(DeactivateTaxConfigurationRequest { tax_config_id: id }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

#[derive(Deserialize)]
pub struct ApplicableTaxParams {
    pub location_id:  i32,
    pub category_id:  Option<i32>,
}

pub async fn get_applicable_tax_configurations(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Query(params): Query<ApplicableTaxParams>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            let has_category = params.category_id.is_some();
            state.catalog_client.client().get_applicable_tax_configurations(GetApplicableTaxConfigurationsRequest {
                location_id:  params.location_id,
                category_id:  params.category_id.unwrap_or(0),
                has_category,
            }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

#[derive(Deserialize)]
pub struct CalculateTaxParams {
    pub location_id:  i32,
    pub category_id:  Option<i32>,
    pub amount:       f64,
}

pub async fn calculate_tax(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Query(params): Query<CalculateTaxParams>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            let has_category = params.category_id.is_some();
            state.catalog_client.client().calculate_tax(CalculateTaxRequest {
                location_id:  params.location_id,
                category_id:  params.category_id.unwrap_or(0),
                has_category,
                amount:       params.amount,
            }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}
