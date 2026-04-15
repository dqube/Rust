//! Product pass-through handlers — REST → gRPC proxy.
//!
//! Each handler calls product-service via the tonic-generated
//! `ProductServiceClient`. Proto types carry serde + utoipa derives (from
//! `build.rs`) so they serve as both JSON DTOs and OpenAPI schemas.
//!
//! Trace context (`traceparent`, `x-request-id`) is propagated to downstream
//! gRPC calls via the task-local [`TRACE_CTX`]. Mutating operations emit
//! structured audit events.

use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use ddd_bff::prelude::*;
use crate::proto::product_service_client::ProductServiceClient;
use crate::proto::{
    ConfirmImageUploadRequest, CreateProductRequest, CreateProductResponse,
    DeactivateProductRequest, GetProductRequest, GetProductResponse,
    ListProductsRequest, RequestImageUploadUrlRequest,
    UpdateStockRequest,
};

// re-export for OpenAPI schema reference in main.rs
pub use crate::proto;

/// Shared state for product handlers.
pub type ProductState = Arc<ProductClient>;

/// Wraps a tonic channel for constructing product-service gRPC clients.
#[derive(Clone)]
pub struct ProductClient {
    channel: tonic::transport::Channel,
}

impl ProductClient {
    pub fn new(channel: tonic::transport::Channel) -> Self {
        Self { channel }
    }

    pub fn client(
        &self,
    ) -> ProductServiceClient<
        tonic::service::interceptor::InterceptedService<tonic::transport::Channel, TracingInterceptor>,
    > {
        ProductServiceClient::with_interceptor(self.channel.clone(), TracingInterceptor)
    }
}

// -- Helpers ------------------------------------------------------------------

/// Actor identity from JWT claims (or "anonymous" when auth is disabled).
fn actor_from_ext(claims: &Option<ddd_shared_kernel::jwt::StandardClaims>) -> &str {
    claims
        .as_ref()
        .map(|c| c.sub.as_str())
        .unwrap_or("anonymous")
}

// -- Handlers -----------------------------------------------------------------

/// Create a new product.
#[utoipa::path(
    post,
    path = "/admin/products",
    request_body = CreateProductRequest,
    responses(
        (status = 201, description = "Product created", body = CreateProductResponse),
        (status = 400, description = "Invalid request", body = ProblemDetail),
    ),
    tag = "Products"
)]
pub async fn create_product(
    State(state): State<ProductState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Json(req): Json<CreateProductRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state.client().create_product(req).await
        })
        .await
        .into_problem()?;
    let inner = resp.into_inner();

    audit(AuditEvent {
        action: "create_product",
        resource: "product",
        resource_id: &inner.id,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });

    Ok((StatusCode::CREATED, Json(inner)))
}

/// Get a product by ID.
#[utoipa::path(
    get,
    path = "/admin/products/{id}",
    params(("id" = String, Path, description = "Product ID (UUID)")),
    responses(
        (status = 200, description = "Product found", body = GetProductResponse),
        (status = 404, description = "Product not found", body = ProblemDetail),
    ),
    tag = "Products"
)]
pub async fn get_product(
    State(state): State<ProductState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.client().get_product(GetProductRequest { id }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

/// List products with pagination.
#[utoipa::path(
    get,
    path = "/admin/products",
    params(
        ("page" = Option<u32>, Query, description = "Page number (default: 1)"),
        ("per_page" = Option<u32>, Query, description = "Items per page (default: 20)"),
    ),
    responses(
        (status = 200, description = "Paginated list of products", body = proto::ListProductsResponse),
    ),
    tag = "Products"
)]
pub async fn list_products(
    State(state): State<ProductState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Query(params): Query<ListQuery>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .client()
                .list_products(ListProductsRequest {
                    page: params.page.unwrap_or(0),
                    per_page: params.per_page.unwrap_or(0),
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

/// Update product stock.
#[utoipa::path(
    put,
    path = "/admin/products/{id}/stock",
    params(("id" = String, Path, description = "Product ID (UUID)")),
    request_body = StockBody,
    responses(
        (status = 204, description = "Stock updated"),
        (status = 404, description = "Product not found", body = ProblemDetail),
        (status = 409, description = "Product is inactive", body = ProblemDetail),
    ),
    tag = "Products"
)]
pub async fn update_stock(
    State(state): State<ProductState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(id): Path<String>,
    Json(body): Json<StockBody>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .client()
                .update_stock(UpdateStockRequest {
                    id: id.clone(),
                    stock: body.stock,
                })
                .await
        })
        .await
        .into_problem()?;

    audit(AuditEvent {
        action: "update_stock",
        resource: "product",
        resource_id: &id,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: Some(&format!("stock={}", body.stock)),
    });

    Ok(StatusCode::NO_CONTENT)
}

/// Deactivate a product.
#[utoipa::path(
    put,
    path = "/admin/products/{id}/deactivate",
    params(("id" = String, Path, description = "Product ID (UUID)")),
    responses(
        (status = 204, description = "Product deactivated"),
        (status = 404, description = "Product not found", body = ProblemDetail),
        (status = 409, description = "Product is already inactive", body = ProblemDetail),
    ),
    tag = "Products"
)]
pub async fn deactivate_product(
    State(state): State<ProductState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .client()
                .deactivate_product(DeactivateProductRequest { id: id.clone() })
                .await
        })
        .await
        .into_problem()?;

    audit(AuditEvent {
        action: "deactivate_product",
        resource: "product",
        resource_id: &id,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });

    Ok(StatusCode::NO_CONTENT)
}

// -- Helper types for REST extraction -----------------------------------------

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct StockBody {
    pub stock: u32,
}

/// Request body for `POST /admin/products/{id}/image-upload-url`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ImageUploadUrlBody {
    /// Original filename, e.g. `"hero.jpg"`.
    pub filename: String,
    /// MIME type, e.g. `"image/jpeg"`.
    pub content_type: String,
}

/// Request body for `POST /admin/products/{id}/confirm-image`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ConfirmImageBody {
    /// Public URL of the uploaded blob returned by the storage provider.
    pub image_url: String,
}

// -- Image upload handlers ----------------------------------------------------

/// Request a presigned upload URL for a product image.
///
/// The client should PUT the image directly to the returned `upload_url` using
/// the matching `Content-Type`.  No image data passes through the BFF.
#[utoipa::path(
    post,
    path = "/admin/products/{id}/image-upload-url",
    params(("id" = String, Path, description = "Product ID (UUID)")),
    request_body = ImageUploadUrlBody,
    responses(
        (status = 200, description = "Presigned URL issued", body = RequestImageUploadUrlResponse),
        (status = 404, description = "Product not found", body = ProblemDetail),
    ),
    tag = "Products"
)]
pub async fn request_image_upload_url(
    State(state): State<ProductState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(body): Json<ImageUploadUrlBody>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .client()
                .request_image_upload_url(RequestImageUploadUrlRequest {
                    product_id: id,
                    filename: body.filename,
                    content_type: body.content_type,
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

/// Confirm that a product image upload succeeded.
///
/// Call this after the client successfully PUT the image to the presigned URL.
/// The `image_url` should be the public URL of the uploaded object.
#[utoipa::path(
    post,
    path = "/admin/products/{id}/confirm-image",
    params(("id" = String, Path, description = "Product ID (UUID)")),
    request_body = ConfirmImageBody,
    responses(
        (status = 204, description = "Image confirmed"),
        (status = 404, description = "Product not found", body = ProblemDetail),
        (status = 409, description = "Product is inactive", body = ProblemDetail),
    ),
    tag = "Products"
)]
pub async fn confirm_image_upload(
    State(state): State<ProductState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(id): Path<String>,
    Json(body): Json<ConfirmImageBody>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .client()
                .confirm_image_upload(ConfirmImageUploadRequest {
                    product_id: id.clone(),
                    image_url: body.image_url,
                })
                .await
        })
        .await
        .into_problem()?;

    audit(AuditEvent {
        action: "confirm_image_upload",
        resource: "product",
        resource_id: &id,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });

    Ok(StatusCode::NO_CONTENT)
}

// re-export new response types for OpenAPI schema registration
pub use crate::proto::RequestImageUploadUrlResponse;
