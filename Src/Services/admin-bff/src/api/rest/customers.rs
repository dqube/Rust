//! Customer pass-through handlers — REST → gRPC proxy.
//!
//! Each handler calls customer-service via the tonic-generated
//! `CustomerServiceClient`. Trace context is propagated via the task-local
//! [`TRACE_CTX`].

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use ddd_bff::prelude::*;

use crate::application::state::AppState;
use crate::proto_customer::{
    AddCustomerAddressRequest, AddLoyaltyPointsRequest, AddToWishlistRequest, ClearWishlistRequest,
    ConfirmAvatarUploadRequest, CreateCustomerProfileRequest, CreateCustomerRequest,
    EnsureCustomerProfileRequest, GetCustomerAvatarUrlRequest, GetCustomerProfileRequest,
    GetCustomerRequest, GetWishlistRequest, ListCustomersRequest, RedeemLoyaltyPointsRequest,
    RejectKycRequest, RemoveCustomerAddressRequest, RemoveFromWishlistRequest,
    RequestAvatarUploadUrlRequest, RequestKycDocumentUploadUrlRequest,
    SetDefaultCustomerAddressRequest, SubmitForKycReviewRequest, SubmitKycDocumentRequest,
    UpdateCustomerAddressRequest, UpdateCustomerInfoRequest, UpdateCustomerProfileRequest,
    UpdateNotificationPreferencesRequest, VerifyKycRequest,
};

// ── Customer CRUD ─────────────────────────────────────────────────────────────

pub async fn create_customer(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<CreateCustomerRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.customer_client.client().create_customer(req).await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn get_customer(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .get_customer(GetCustomerRequest { customer_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_customer_by_user_id(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(user_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .get_customer_by_user_id(crate::proto_customer::GetCustomerByUserIdRequest {
                    user_id,
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

#[derive(Deserialize)]
pub struct ListCustomersParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub search: Option<String>,
    pub country_code: Option<String>,
}

pub async fn list_customers(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Query(params): Query<ListCustomersParams>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .list_customers(ListCustomersRequest {
                    page: params.page.unwrap_or(1),
                    per_page: params.per_page.unwrap_or(20),
                    search: params.search.unwrap_or_default(),
                    country_code: params.country_code.unwrap_or_default(),
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn update_customer_info(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
    Json(mut req): Json<UpdateCustomerInfoRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.customer_id = customer_id;
    TRACE_CTX
        .scope(trace_ctx, async {
            state.customer_client.client().update_customer_info(req).await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn ensure_customer_profile(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<EnsureCustomerProfileRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .ensure_customer_profile(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

// ── Loyalty ───────────────────────────────────────────────────────────────────

pub async fn add_loyalty_points(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
    Json(mut req): Json<AddLoyaltyPointsRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.customer_id = customer_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.customer_client.client().add_loyalty_points(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn redeem_loyalty_points(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
    Json(mut req): Json<RedeemLoyaltyPointsRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.customer_id = customer_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .redeem_loyalty_points(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

// ── Addresses ────────────────────────────────────────────────────────────────

pub async fn add_customer_address(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
    Json(mut req): Json<AddCustomerAddressRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.customer_id = customer_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .add_customer_address(req)
                .await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn update_customer_address(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path((customer_id, address_id)): Path<(String, String)>,
    Json(mut req): Json<UpdateCustomerAddressRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.customer_id = customer_id;
    req.address_id = address_id;
    TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .update_customer_address(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_customer_address(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path((customer_id, address_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .remove_customer_address(RemoveCustomerAddressRequest {
                    customer_id,
                    address_id,
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn set_default_customer_address(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path((customer_id, address_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .set_default_customer_address(SetDefaultCustomerAddressRequest {
                    customer_id,
                    address_id,
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Avatar ────────────────────────────────────────────────────────────────────

pub async fn request_avatar_upload_url(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
    Json(mut req): Json<RequestAvatarUploadUrlRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.customer_id = customer_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .request_avatar_upload_url(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn confirm_avatar_upload(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
    Json(mut req): Json<ConfirmAvatarUploadRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.customer_id = customer_id;
    TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .confirm_avatar_upload(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_customer_avatar_url(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .get_customer_avatar_url(GetCustomerAvatarUrlRequest { customer_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

// ── Profile & KYC ────────────────────────────────────────────────────────────

pub async fn create_customer_profile(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
    Json(mut req): Json<CreateCustomerProfileRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.customer_id = customer_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .create_customer_profile(req)
                .await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn get_customer_profile(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .get_customer_profile(GetCustomerProfileRequest { customer_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn update_customer_profile(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
    Json(mut req): Json<UpdateCustomerProfileRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.customer_id = customer_id;
    TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .update_customer_profile(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_notification_preferences(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
    Json(mut req): Json<UpdateNotificationPreferencesRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.customer_id = customer_id;
    TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .update_notification_preferences(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn submit_kyc_document(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
    Json(mut req): Json<SubmitKycDocumentRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.customer_id = customer_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .submit_kyc_document(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn request_kyc_document_upload_url(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
    Json(mut req): Json<RequestKycDocumentUploadUrlRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.customer_id = customer_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .request_kyc_document_upload_url(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn submit_for_kyc_review(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .submit_for_kyc_review(SubmitForKycReviewRequest { customer_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn verify_kyc(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
    Json(mut req): Json<VerifyKycRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.customer_id = customer_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.customer_client.client().verify_kyc(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn reject_kyc(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
    Json(mut req): Json<RejectKycRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.customer_id = customer_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.customer_client.client().reject_kyc(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

// ── Wishlist ──────────────────────────────────────────────────────────────────

pub async fn get_wishlist(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .get_wishlist(GetWishlistRequest { customer_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn add_to_wishlist(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
    Json(mut req): Json<AddToWishlistRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.customer_id = customer_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.customer_client.client().add_to_wishlist(req).await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn remove_from_wishlist(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path((customer_id, product_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .remove_from_wishlist(RemoveFromWishlistRequest {
                    customer_id,
                    product_id,
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn clear_wishlist(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(customer_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx, async {
            state
                .customer_client
                .client()
                .clear_wishlist(ClearWishlistRequest { customer_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}
