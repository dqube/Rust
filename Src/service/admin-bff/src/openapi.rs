//! OpenAPI + Scalar UI for the Admin BFF.
//!
//! [`AdminApiDoc`] declares all schemas used by this service. The generic
//! [`openapi_router`] that serves the Scalar UI and JSON spec lives in
//! `ddd-bff` and is re-exported here for convenience.

use utoipa::OpenApi;

use ddd_bff::transcode::ProblemDetail;

use crate::aggregation::{BatchRequest, BatchResponse, BatchResult};
use crate::handlers::products::{ConfirmImageBody, ImageUploadUrlBody};
use crate::proto::{
    ConfirmImageUploadRequest, ConfirmImageUploadResponse, CreateProductRequest,
    CreateProductResponse, DeactivateProductRequest, DeactivateProductResponse, GetProductRequest,
    GetProductResponse, ListProductsRequest, ListProductsResponse, Product,
    RequestImageUploadUrlRequest, RequestImageUploadUrlResponse, UpdateStockRequest,
    UpdateStockResponse,
};
use crate::proto_order::{
    CancelOrderRequest, CancelOrderResponse, ConfirmOrderRequest, ConfirmOrderResponse,
    CreateOrderRequest, CreateOrderResponse, GetOrderRequest, GetOrderResponse,
    ListOrdersRequest, ListOrdersResponse, Order, OrderItem,
};

/// Re-export the generic OpenAPI / Scalar router from `ddd-bff`.
pub use ddd_bff::openapi::openapi_router;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Admin BFF API",
        description = "Admin Backend-for-Frontend — proxies admin operations to downstream services",
        version = "0.1.0"
    ),
    components(schemas(
        // Aggregation DTOs
        BatchRequest,
        BatchResponse,
        BatchResult,
        // Problem detail
        ProblemDetail,
        // Product proto types (referenced by passthrough routes)
        Product,
        CreateProductRequest,
        CreateProductResponse,
        GetProductRequest,
        GetProductResponse,
        ListProductsRequest,
        ListProductsResponse,
        UpdateStockRequest,
        UpdateStockResponse,
        DeactivateProductRequest,
        DeactivateProductResponse,
        RequestImageUploadUrlRequest,
        RequestImageUploadUrlResponse,
        ConfirmImageUploadRequest,
        ConfirmImageUploadResponse,
        ImageUploadUrlBody,
        ConfirmImageBody,
        // Order proto types (referenced by passthrough routes)
        Order,
        OrderItem,
        CreateOrderRequest,
        CreateOrderResponse,
        GetOrderRequest,
        GetOrderResponse,
        ListOrdersRequest,
        ListOrdersResponse,
        ConfirmOrderRequest,
        ConfirmOrderResponse,
        CancelOrderRequest,
        CancelOrderResponse,
    )),
)]
pub struct AdminApiDoc;
