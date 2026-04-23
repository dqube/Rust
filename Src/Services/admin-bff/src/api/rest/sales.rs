//! Sales + Returns pass-through handlers — REST → gRPC proxy.

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use ddd_bff::prelude::*;

use crate::application::state::AppState;
use crate::proto_sales::{
    AddReturnDetailRequest, AddSaleDetailRequest, ApplyDiscountRequest, CancelSaleRequest,
    CompleteSaleRequest, CreateReturnRequest, CreateSaleRequest, GetReturnByIdRequest,
    GetReturnsBySaleRequest, GetReturnsByCustomerRequest, GetReturnsByEmployeeRequest,
    GetSaleByIdRequest, GetSaleByReceiptRequest, GetSaleReceiptUrlRequest, GetSalesRequest,
    GetSalesByCustomerRequest, GetSalesByEmployeeRequest, GetSalesByStoreRequest,
    ProcessReturnRequest, RemoveSaleDetailRequest, UpdateSaleDetailRequest, UpdateSaleStatusRequest,
    UploadSaleReceiptRequest,
};

// ── Sales ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GetSalesParams {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
    pub status: Option<String>,
}

pub async fn list_sales(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Query(params): Query<GetSalesParams>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .sales_client
                .client()
                .get_sales(GetSalesRequest {
                    page: params.page.unwrap_or(1),
                    page_size: params.page_size.unwrap_or(20),
                    status: params.status.unwrap_or_default(),
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn create_sale(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<CreateSaleRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.sales_client.client().create_sale(req).await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn get_sale(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(sale_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .sales_client
                .client()
                .get_sale_by_id(GetSaleByIdRequest { sale_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_sale_by_receipt(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(receipt_number): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .sales_client
                .client()
                .get_sale_by_receipt(GetSaleByReceiptRequest { receipt_number })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_sales_by_store(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(store_id): Path<i32>,
    Query(params): Query<DateRangeParams>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .sales_client
                .client()
                .get_sales_by_store(GetSalesByStoreRequest {
                    store_id,
                    from_date: params.from_date.unwrap_or_default(),
                    to_date: params.to_date.unwrap_or_default(),
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_sales_by_employee(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(employee_id): Path<String>,
    Query(params): Query<DateRangeParams>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .sales_client
                .client()
                .get_sales_by_employee(GetSalesByEmployeeRequest {
                    employee_id,
                    from_date: params.from_date.unwrap_or_default(),
                    to_date: params.to_date.unwrap_or_default(),
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_sales_by_customer(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .sales_client
                .client()
                .get_sales_by_customer(GetSalesByCustomerRequest { customer_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn add_sale_detail(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(sale_id): Path<String>,
    Json(mut req): Json<AddSaleDetailRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.sale_id = sale_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.sales_client.client().add_sale_detail(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn update_sale_detail(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path((sale_id, sale_detail_id)): Path<(String, String)>,
    Json(mut req): Json<UpdateSaleDetailRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.sale_id = sale_id;
    req.sale_detail_id = sale_detail_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.sales_client.client().update_sale_detail(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn remove_sale_detail(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path((sale_id, sale_detail_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .sales_client
                .client()
                .remove_sale_detail(RemoveSaleDetailRequest {
                    sale_id,
                    sale_detail_id,
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn apply_discount(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(sale_id): Path<String>,
    Json(mut req): Json<ApplyDiscountRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.sale_id = sale_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.sales_client.client().apply_discount(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn complete_sale(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(sale_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .sales_client
                .client()
                .complete_sale(CompleteSaleRequest { sale_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn cancel_sale(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(sale_id): Path<String>,
    Json(req): Json<CancelSaleBody>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .sales_client
                .client()
                .cancel_sale(CancelSaleRequest {
                    sale_id,
                    reason: req.reason,
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn update_sale_status(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(sale_id): Path<String>,
    Json(req): Json<UpdateSaleStatusBody>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .sales_client
                .client()
                .update_sale_status(UpdateSaleStatusRequest {
                    sale_id,
                    status: req.status,
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_sale_receipt_url(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(sale_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .sales_client
                .client()
                .get_sale_receipt_url(GetSaleReceiptUrlRequest { sale_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn upload_sale_receipt(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(sale_id): Path<String>,
    Json(req): Json<UploadReceiptBody>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .sales_client
                .client()
                .upload_sale_receipt(UploadSaleReceiptRequest {
                    sale_id,
                    file_content: req.file_content,
                    file_name: req.file_name,
                    content_type: req.content_type,
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

// ── Returns ───────────────────────────────────────────────────────────────────

pub async fn create_return(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<CreateReturnRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.sales_client.client().create_return(req).await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn get_return(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(return_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .sales_client
                .client()
                .get_return_by_id(GetReturnByIdRequest { return_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_returns_by_sale(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(sale_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .sales_client
                .client()
                .get_returns_by_sale(GetReturnsBySaleRequest { sale_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_returns_by_employee(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(employee_id): Path<String>,
    Query(params): Query<DateRangeParams>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .sales_client
                .client()
                .get_returns_by_employee(GetReturnsByEmployeeRequest {
                    employee_id,
                    from_date: params.from_date.unwrap_or_default(),
                    to_date: params.to_date.unwrap_or_default(),
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_returns_by_customer(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .sales_client
                .client()
                .get_returns_by_customer(GetReturnsByCustomerRequest { customer_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn add_return_detail(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(return_id): Path<String>,
    Json(mut req): Json<AddReturnDetailRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.return_id = return_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.sales_client.client().add_return_detail(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn process_return(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(return_id): Path<String>,
    Json(req): Json<ProcessReturnBody>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .sales_client
                .client()
                .process_return(ProcessReturnRequest {
                    return_id,
                    total_refund: req.total_refund,
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

// ── Request body types ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DateRangeParams {
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

#[derive(Deserialize)]
pub struct CancelSaleBody {
    pub reason: String,
}

#[derive(Deserialize)]
pub struct UpdateSaleStatusBody {
    pub status: String,
}

#[derive(Deserialize)]
pub struct UploadReceiptBody {
    pub file_content: Vec<u8>,
    pub file_name: String,
    pub content_type: String,
}

#[derive(Deserialize)]
pub struct ProcessReturnBody {
    pub total_refund: String,
}
