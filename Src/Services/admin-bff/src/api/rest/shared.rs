//! Shared reference-data pass-through handlers — REST → gRPC proxy.
//!
//! Each handler calls shared-service via the tonic-generated
//! `SharedServiceClient`. Trace context (`traceparent`, `x-request-id`) is
//! propagated to downstream gRPC calls via the task-local [`TRACE_CTX`].
//! Mutating operations emit structured audit events.

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

use ddd_bff::prelude::*;

use crate::application::state::AppState;
use crate::proto_shared::{
    CreateCityRequest, CreateCountryRequest, CreateCurrencyRequest, CreatePincodeRequest,
    CreateStateRequest, Empty, GetByCodeRequest, UpdateCityRequest, UpdateCountryRequest,
    UpdateCurrencyRequest, UpdatePincodeRequest, UpdateStateRequest,
};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn actor_from_ext(claims: &Option<ddd_shared_kernel::jwt::StandardClaims>) -> &str {
    claims.as_ref().map(|c| c.sub.as_str()).unwrap_or("anonymous")
}

// ─── Currencies ──────────────────────────────────────────────────────────────

pub async fn list_currencies(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.shared_client.client().get_all_currencies(Empty {}).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_currency(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .shared_client
                .client()
                .get_currency_by_code(GetByCodeRequest { code })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn create_currency(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Json(req): Json<CreateCurrencyRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let code = req.code.clone();
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state.shared_client.client().create_currency(req).await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "create_currency",
        resource: "currency",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::CREATED)
}

pub async fn update_currency(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
    Json(mut req): Json<UpdateCurrencyRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.code = code.clone();
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state.shared_client.client().update_currency(req).await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "update_currency",
        resource: "currency",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_currency(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .shared_client
                .client()
                .delete_currency(GetByCodeRequest { code: code.clone() })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "delete_currency",
        resource: "currency",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn activate_currency(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .shared_client
                .client()
                .activate_currency(GetByCodeRequest { code: code.clone() })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "activate_currency",
        resource: "currency",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn deactivate_currency(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .shared_client
                .client()
                .deactivate_currency(GetByCodeRequest { code: code.clone() })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "deactivate_currency",
        resource: "currency",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

// ─── Countries ───────────────────────────────────────────────────────────────

pub async fn list_countries(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.shared_client.client().get_all_countries(Empty {}).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_country(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .shared_client
                .client()
                .get_country_by_code(GetByCodeRequest { code })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn list_countries_by_currency(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(currency_code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .shared_client
                .client()
                .get_countries_by_currency(GetByCodeRequest { code: currency_code })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn create_country(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Json(req): Json<CreateCountryRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let code = req.code.clone();
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state.shared_client.client().create_country(req).await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "create_country",
        resource: "country",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::CREATED)
}

pub async fn update_country(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
    Json(mut req): Json<UpdateCountryRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.code = code.clone();
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state.shared_client.client().update_country(req).await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "update_country",
        resource: "country",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_country(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .shared_client
                .client()
                .delete_country(GetByCodeRequest { code: code.clone() })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "delete_country",
        resource: "country",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn activate_country(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .shared_client
                .client()
                .activate_country(GetByCodeRequest { code: code.clone() })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "activate_country",
        resource: "country",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn deactivate_country(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .shared_client
                .client()
                .deactivate_country(GetByCodeRequest { code: code.clone() })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "deactivate_country",
        resource: "country",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

// ─── States ──────────────────────────────────────────────────────────────────

pub async fn list_states(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.shared_client.client().get_all_states(Empty {}).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_state(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .shared_client
                .client()
                .get_state_by_code(GetByCodeRequest { code })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn list_states_by_country(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(country_code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .shared_client
                .client()
                .get_states_by_country(GetByCodeRequest { code: country_code })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn create_state(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Json(req): Json<CreateStateRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let code = req.code.clone();
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state.shared_client.client().create_state(req).await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "create_state",
        resource: "state",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::CREATED)
}

pub async fn update_state(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
    Json(mut req): Json<UpdateStateRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.code = code.clone();
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state.shared_client.client().update_state(req).await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "update_state",
        resource: "state",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_state(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .shared_client
                .client()
                .delete_state(GetByCodeRequest { code: code.clone() })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "delete_state",
        resource: "state",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn activate_state(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .shared_client
                .client()
                .activate_state(GetByCodeRequest { code: code.clone() })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "activate_state",
        resource: "state",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn deactivate_state(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .shared_client
                .client()
                .deactivate_state(GetByCodeRequest { code: code.clone() })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "deactivate_state",
        resource: "state",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

// ─── Cities ──────────────────────────────────────────────────────────────────

pub async fn list_cities(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.shared_client.client().get_all_cities(Empty {}).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_city(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .shared_client
                .client()
                .get_city_by_code(GetByCodeRequest { code })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn list_cities_by_state(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(state_code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .shared_client
                .client()
                .get_cities_by_state(GetByCodeRequest { code: state_code })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn create_city(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Json(req): Json<CreateCityRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let code = req.code.clone();
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state.shared_client.client().create_city(req).await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "create_city",
        resource: "city",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::CREATED)
}

pub async fn update_city(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
    Json(mut req): Json<UpdateCityRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.code = code.clone();
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state.shared_client.client().update_city(req).await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "update_city",
        resource: "city",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_city(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .shared_client
                .client()
                .delete_city(GetByCodeRequest { code: code.clone() })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "delete_city",
        resource: "city",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn activate_city(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .shared_client
                .client()
                .activate_city(GetByCodeRequest { code: code.clone() })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "activate_city",
        resource: "city",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn deactivate_city(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .shared_client
                .client()
                .deactivate_city(GetByCodeRequest { code: code.clone() })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "deactivate_city",
        resource: "city",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

// ─── Pincodes ────────────────────────────────────────────────────────────────

pub async fn list_pincodes(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.shared_client.client().get_all_pincodes(Empty {}).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_pincode(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .shared_client
                .client()
                .get_pincode_by_code(GetByCodeRequest { code })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn list_pincodes_by_city(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(city_code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .shared_client
                .client()
                .get_pincodes_by_city(GetByCodeRequest { code: city_code })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn create_pincode(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Json(req): Json<CreatePincodeRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let code = req.code.clone();
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state.shared_client.client().create_pincode(req).await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "create_pincode",
        resource: "pincode",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::CREATED)
}

pub async fn update_pincode(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
    Json(mut req): Json<UpdatePincodeRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.code = code.clone();
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state.shared_client.client().update_pincode(req).await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "update_pincode",
        resource: "pincode",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_pincode(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .shared_client
                .client()
                .delete_pincode(GetByCodeRequest { code: code.clone() })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "delete_pincode",
        resource: "pincode",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn activate_pincode(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .shared_client
                .client()
                .activate_pincode(GetByCodeRequest { code: code.clone() })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "activate_pincode",
        resource: "pincode",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn deactivate_pincode(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx.clone(), async {
            state
                .shared_client
                .client()
                .deactivate_pincode(GetByCodeRequest { code: code.clone() })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "deactivate_pincode",
        resource: "pincode",
        resource_id: &code,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &trace_ctx.request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}
