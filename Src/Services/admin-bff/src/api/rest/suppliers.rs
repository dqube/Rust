//! Supplier pass-through handlers — REST → gRPC proxy.

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use ddd_bff::prelude::*;

use crate::application::state::AppState;
use crate::proto_supplier::{
    ActivateSupplierRequest, AddSupplierProductRequest, CancelPurchaseOrderRequest,
    ConfirmDocumentUploadRequest, CreatePurchaseOrderRequest, CreateSupplierContactRequest,
    CreateSupplierRequest, DeactivateSupplierRequest, DeleteSupplierDocumentRequest,
    DeleteSupplierRequest, GetPurchaseOrderRequest, GetSupplierAddressesRequest,
    GetSupplierContactsRequest, GetSupplierDocumentsRequest, GetSupplierRequest,
    ListPurchaseOrdersRequest, ListSupplierProductsRequest, ListSuppliersRequest,
    RemoveSupplierProductRequest, RequestDocumentUploadUrlRequest, SubmitPurchaseOrderRequest,
    UpdateOnboardingStatusRequest, UpdateSupplierRequest, UpdateSupplierStatusRequest,
};

// ── Suppliers ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListSuppliersParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub active_only: Option<bool>,
    pub search: Option<String>,
}

pub async fn list_suppliers(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Query(params): Query<ListSuppliersParams>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .list_suppliers(ListSuppliersRequest {
                    page: params.page.unwrap_or(1),
                    per_page: params.per_page.unwrap_or(20),
                    active_only: params.active_only.unwrap_or(false),
                    search: params.search.unwrap_or_default(),
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn create_supplier(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<CreateSupplierRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.supplier_client.client().create_supplier(req).await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn get_supplier(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .get_supplier(GetSupplierRequest { id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn update_supplier(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<UpdateSupplierRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.supplier_client.client().update_supplier(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn activate_supplier(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<ActivateSupplierRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.supplier_client.client().activate_supplier(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn deactivate_supplier(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<DeactivateSupplierRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .deactivate_supplier(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn delete_supplier(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .delete_supplier(DeleteSupplierRequest { id })
                .await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_supplier_status(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<UpdateSupplierStatusRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .update_supplier_status(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn update_onboarding_status(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<UpdateOnboardingStatusRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .update_onboarding_status(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

// ── Addresses ────────────────────────────────────────────────────────────────

pub async fn get_supplier_addresses(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(supplier_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .get_supplier_addresses(GetSupplierAddressesRequest { supplier_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

// ── Contacts ─────────────────────────────────────────────────────────────────

pub async fn get_supplier_contacts(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(supplier_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .get_supplier_contacts(GetSupplierContactsRequest { supplier_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn create_supplier_contact(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(supplier_id): Path<String>,
    Json(mut req): Json<CreateSupplierContactRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.supplier_id = supplier_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .create_supplier_contact(req)
                .await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

// ── Documents ────────────────────────────────────────────────────────────────

pub async fn get_supplier_documents(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(supplier_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .get_supplier_documents(GetSupplierDocumentsRequest { supplier_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn request_document_upload_url(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(supplier_id): Path<String>,
    Json(mut req): Json<RequestDocumentUploadUrlRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.supplier_id = supplier_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .request_document_upload_url(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn confirm_document_upload(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(supplier_id): Path<String>,
    Json(mut req): Json<ConfirmDocumentUploadRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.supplier_id = supplier_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .confirm_document_upload(req)
                .await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn delete_supplier_document(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path((supplier_id, document_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .delete_supplier_document(DeleteSupplierDocumentRequest {
                    supplier_id,
                    document_id,
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Supplier Products ─────────────────────────────────────────────────────────

pub async fn list_supplier_products(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(supplier_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .list_supplier_products(ListSupplierProductsRequest { supplier_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn add_supplier_product(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(supplier_id): Path<String>,
    Json(mut req): Json<AddSupplierProductRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.supplier_id = supplier_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .add_supplier_product(req)
                .await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn remove_supplier_product(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path((supplier_id, supplier_product_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .remove_supplier_product(RemoveSupplierProductRequest {
                    supplier_id,
                    supplier_product_id,
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Purchase Orders ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListPurchaseOrdersParams {
    pub supplier_id: Option<String>,
    pub store_id: Option<i32>,
    pub status: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

pub async fn list_purchase_orders(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Query(params): Query<ListPurchaseOrdersParams>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .list_purchase_orders(ListPurchaseOrdersRequest {
                    supplier_id: params.supplier_id.unwrap_or_default(),
                    store_id: params.store_id.unwrap_or(0),
                    status: params.status.unwrap_or_default(),
                    from_date: params.from_date.unwrap_or_default(),
                    to_date: params.to_date.unwrap_or_default(),
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn create_purchase_order(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<CreatePurchaseOrderRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .create_purchase_order(req)
                .await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn get_purchase_order(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .get_purchase_order(GetPurchaseOrderRequest { id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn submit_purchase_order(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<SubmitPurchaseOrderRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .submit_purchase_order(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn cancel_purchase_order(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<CancelPurchaseOrderRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .supplier_client
                .client()
                .cancel_purchase_order(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}
