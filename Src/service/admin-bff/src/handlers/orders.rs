//! Order pass-through handlers — REST → gRPC proxy.
//!
//! Each handler calls order-service via the tonic-generated
//! `OrderServiceClient`. Proto types carry serde + utoipa derives (from
//! `build.rs`) so they serve as both JSON DTOs and OpenAPI schemas.

use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use ddd_bff::prelude::*;
use crate::proto_order::order_service_client::OrderServiceClient;
use crate::proto_order::{
    CancelOrderRequest, ConfirmOrderRequest, CreateOrderRequest, GetOrderRequest,
    ListOrdersRequest,
};

/// Shared state for order handlers.
pub type OrderState = Arc<OrderClient>;

/// Wraps a tonic channel for constructing order-service gRPC clients.
#[derive(Clone)]
pub struct OrderClient {
    channel: tonic::transport::Channel,
}

impl OrderClient {
    pub fn new(channel: tonic::transport::Channel) -> Self {
        Self { channel }
    }

    pub fn client(
        &self,
    ) -> OrderServiceClient<
        tonic::service::interceptor::InterceptedService<tonic::transport::Channel, TracingInterceptor>,
    > {
        OrderServiceClient::with_interceptor(self.channel.clone(), TracingInterceptor)
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

/// Place a new order.
pub async fn create_order(
    State(state): State<OrderState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Json(req): Json<CreateOrderRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state.client().create_order(req).await
        })
        .await
        .into_problem()?;
    
    let inner = resp.into_inner();

    audit(AuditEvent {
        action: "create_order",
        resource: "order",
        resource_id: &inner.id,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });

    Ok((StatusCode::CREATED, Json(inner)))
}

/// Fetch an order by ID.
pub async fn get_order(
    State(state): State<OrderState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.client().get_order(GetOrderRequest { id }).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

/// List orders with pagination.
pub async fn list_orders(
    State(state): State<OrderState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Query(params): Query<ListQuery>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .client()
                .list_orders(ListOrdersRequest {
                    page: params.page.unwrap_or(0),
                    per_page: params.per_page.unwrap_or(0),
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

/// Confirm a pending order.
pub async fn confirm_order(
    State(state): State<OrderState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .client()
                .confirm_order(ConfirmOrderRequest { id: id.clone() })
                .await
        })
        .await
        .into_problem()?;

    audit(AuditEvent {
        action: "confirm_order",
        resource: "order",
        resource_id: &id,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });

    Ok(StatusCode::NO_CONTENT)
}

/// Cancel an order with a reason.
pub async fn cancel_order(
    State(state): State<OrderState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(id): Path<String>,
    Json(body): Json<CancelBody>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .client()
                .cancel_order(CancelOrderRequest {
                    id: id.clone(),
                    reason: body.reason.clone(),
                })
                .await
        })
        .await
        .into_problem()?;

    audit(AuditEvent {
        action: "cancel_order",
        resource: "order",
        resource_id: &id,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: Some(&format!("reason: {}", body.reason)),
    });

    Ok(StatusCode::NO_CONTENT)
}

// -- Helper types -------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct CancelBody {
    pub reason: String,
}
