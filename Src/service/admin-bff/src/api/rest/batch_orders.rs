//! Aggregation endpoint — fans out multiple GetOrder gRPC calls in parallel
//! and assembles a partial-failure-tolerant response.

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use futures::future::join_all;
use serde::{Deserialize, Serialize};

use ddd_bff::prelude::*;

use crate::proto_order::GetOrderRequest;

use crate::application::state::AppState;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct BatchRequest {
    pub ids: Vec<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BatchResponse {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub results: Vec<BatchResult>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum BatchResult {
    Ok {
        id: String,
        #[schema(value_type = Object)]
        data: serde_json::Value,
    },
    Error {
        id: String,
        code: u16,
        message: String,
    },
}

/// `POST /admin/orders/batch` — fetch many orders in parallel.
///
/// Body: `{ "ids": ["<uuid>", "<uuid>", ...] }`
/// Returns 200 with per-id success/failure; never fails the whole batch.
pub async fn batch_get_orders(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<BatchRequest>,
) -> impl IntoResponse {
    let max = 100usize;
    if req.ids.len() > max {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "type": "urn:problem-type:validation-error",
                "title": "Too many ids",
                "status": 400,
                "detail": format!("max {max} ids per batch, got {}", req.ids.len()),
            })),
        )
            .into_response();
    }

    let fetches = req.ids.into_iter().map(|id| {
        let state = state.clone();
        async move { fetch_one(&state, id).await }
    });

    let results: Vec<BatchResult> = TRACE_CTX
        .scope(trace_ctx, async { join_all(fetches).await })
        .await;
    let succeeded = results.iter().filter(|r| matches!(r, BatchResult::Ok { .. })).count();
    let failed = results.len() - succeeded;

    Json(BatchResponse {
        total: results.len(),
        succeeded,
        failed,
        results,
    })
    .into_response()
}

async fn fetch_one(state: &AppState, id: String) -> BatchResult {
    let result = state
        .order_client
        .client()
        .get_order(GetOrderRequest { id: id.clone() })
        .await;

    match result {
        Ok(resp) => {
            let data = serde_json::to_value(resp.into_inner()).unwrap_or_default();
            BatchResult::Ok { id, data }
        }
        Err(status) => {
            let app_err = ddd_bff::transcode::grpc_status_to_app_error(status);
            BatchResult::Error {
                id,
                code: app_err.http_status_code(),
                message: app_err.to_string(),
            }
        }
    }
}
