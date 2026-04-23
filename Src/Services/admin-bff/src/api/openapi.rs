//! OpenAPI + Scalar UI for the Admin BFF.
//!
//! [`AdminApiDoc`] declares all schemas used by this service. The generic
//! [`openapi_router`] that serves the Scalar UI and JSON spec lives in
//! `ddd-bff` and is re-exported here for convenience.

use utoipa::OpenApi;

use ddd_bff::transcode::ProblemDetail;

use crate::api::rest::batch_orders::{BatchRequest, BatchResponse, BatchResult};
use crate::api::rest::products::{ConfirmImageBody, ImageUploadUrlBody};
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
use crate::proto_sales::{
    AddReturnDetailRequest, AddReturnDetailResponse, AddSaleDetailRequest, AddSaleDetailResponse,
    AppliedDiscountInfo, ApplyDiscountRequest, ApplyDiscountResponse, CancelSaleRequest,
    CancelSaleResponse, CompleteSaleRequest, CompleteSaleResponse, CreateReturnRequest,
    CreateReturnResponse, CreateSaleRequest, CreateSaleResponse, GetReturnByIdRequest,
    GetReturnByIdResponse, GetReturnsBySaleRequest, GetReturnsBySaleResponse,
    GetReturnsByCustomerRequest, GetReturnsByCustomerResponse, GetReturnsByEmployeeRequest,
    GetReturnsByEmployeeResponse, GetSaleByIdRequest, GetSaleByIdResponse, GetSaleByReceiptRequest,
    GetSaleByReceiptResponse, GetSaleReceiptUrlRequest, GetSaleReceiptUrlResponse, GetSalesRequest,
    GetSalesResponse, GetSalesByCustomerRequest, GetSalesByCustomerResponse,
    GetSalesByEmployeeRequest, GetSalesByEmployeeResponse, GetSalesByStoreRequest,
    GetSalesByStoreResponse, ProcessReturnRequest, ProcessReturnResponse, RemoveSaleDetailRequest,
    RemoveSaleDetailResponse, ReturnDetailInfo, ReturnInfo, SaleDetailInfo, SaleInfo,
    UpdateSaleDetailRequest, UpdateSaleDetailResponse, UpdateSaleStatusRequest,
    UpdateSaleStatusResponse, UploadSaleReceiptRequest, UploadSaleReceiptResponse,
};
use crate::proto_shared::{
    CityInfo, CityListResponse, CityResponse, CountryInfo, CountryListResponse, CountryResponse,
    CreateCityRequest, CreateCountryRequest, CreateCurrencyRequest, CreatePincodeRequest,
    CreateStateRequest, CurrencyInfo, CurrencyListResponse, CurrencyResponse, GetByCodeRequest,
    PincodeInfo, PincodeListResponse, PincodeResponse, StateInfo, StateListResponse,
    StateResponse, UpdateCityRequest, UpdateCountryRequest, UpdateCurrencyRequest,
    UpdatePincodeRequest, UpdateStateRequest,
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
        // Shared reference-data proto types (referenced by passthrough routes)
        GetByCodeRequest,
        CurrencyInfo,
        CurrencyResponse,
        CurrencyListResponse,
        CreateCurrencyRequest,
        UpdateCurrencyRequest,
        CountryInfo,
        CountryResponse,
        CountryListResponse,
        CreateCountryRequest,
        UpdateCountryRequest,
        StateInfo,
        StateResponse,
        StateListResponse,
        CreateStateRequest,
        UpdateStateRequest,
        CityInfo,
        CityResponse,
        CityListResponse,
        CreateCityRequest,
        UpdateCityRequest,
        PincodeInfo,
        PincodeResponse,
        PincodeListResponse,
        CreatePincodeRequest,
        UpdatePincodeRequest,
        // Sales proto types
        SaleDetailInfo,
        AppliedDiscountInfo,
        SaleInfo,
        ReturnDetailInfo,
        ReturnInfo,
        CreateSaleRequest,
        CreateSaleResponse,
        AddSaleDetailRequest,
        AddSaleDetailResponse,
        UpdateSaleDetailRequest,
        UpdateSaleDetailResponse,
        RemoveSaleDetailRequest,
        RemoveSaleDetailResponse,
        ApplyDiscountRequest,
        ApplyDiscountResponse,
        CompleteSaleRequest,
        CompleteSaleResponse,
        CancelSaleRequest,
        CancelSaleResponse,
        GetSaleByIdRequest,
        GetSaleByIdResponse,
        GetSaleByReceiptRequest,
        GetSaleByReceiptResponse,
        GetSalesRequest,
        GetSalesResponse,
        UpdateSaleStatusRequest,
        UpdateSaleStatusResponse,
        GetSalesByStoreRequest,
        GetSalesByStoreResponse,
        GetSalesByEmployeeRequest,
        GetSalesByEmployeeResponse,
        GetSalesByCustomerRequest,
        GetSalesByCustomerResponse,
        GetSaleReceiptUrlRequest,
        GetSaleReceiptUrlResponse,
        UploadSaleReceiptRequest,
        UploadSaleReceiptResponse,
        CreateReturnRequest,
        CreateReturnResponse,
        AddReturnDetailRequest,
        AddReturnDetailResponse,
        ProcessReturnRequest,
        ProcessReturnResponse,
        GetReturnByIdRequest,
        GetReturnByIdResponse,
        GetReturnsBySaleRequest,
        GetReturnsBySaleResponse,
        GetReturnsByEmployeeRequest,
        GetReturnsByEmployeeResponse,
        GetReturnsByCustomerRequest,
        GetReturnsByCustomerResponse,
    )),
)]
pub struct AdminApiDoc;
