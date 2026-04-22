//! Auth pass-through handlers — REST → gRPC proxy.
//!
//! Each handler calls auth-service via the tonic-generated
//! `AuthServiceClient`. Trace context is propagated via the task-local
//! [`TRACE_CTX`]. Mutating operations emit structured audit events so
//! admin actions (role assignment, user deactivation, password reset)
//! show up in the audit log.

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

use ddd_bff::prelude::*;
use serde::Deserialize;

use crate::application::state::AppState;
use crate::proto_auth::{
    ActivateUserRequest, AddRolePermissionRequest, AssignRoleRequest, ChangePasswordAdminRequest,
    ChangePasswordRequest, CheckPermissionRequest, CreateRoleRequest, DeactivateUserRequest,
    Empty, ForgotPasswordRequest, GetRolePermissionsByIdRequest, GetRolePermissionsRequest,
    GetUserByEmailRequest, GetUserRequest, ListUserRolesRequest, ListUsersRequest, LoginRequest,
    LogoutRequest, RefreshTokenRequest, RegisterRequest, RemoveRolePermissionRequest,
    RemoveUserRoleRequest, ResetPasswordRequest,
};

// ── Helpers ─────────────────────────────────────────────────────────────────

fn actor_from_ext(claims: &Option<ddd_shared_kernel::jwt::StandardClaims>) -> &str {
    claims.as_ref().map(|c| c.sub.as_str()).unwrap_or("anonymous")
}

// ── Auth flows ──────────────────────────────────────────────────────────────

pub async fn login(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    Json(mut req): Json<LoginRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    if req.ip_address.is_empty() {
        req.ip_address = client_ip.to_string();
    }
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.auth_client.client().login(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn register(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.auth_client.client().register(req).await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn refresh_token(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    Json(mut req): Json<RefreshTokenRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    if req.ip_address.is_empty() {
        req.ip_address = client_ip.to_string();
    }
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.auth_client.client().refresh_token(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn logout(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<LogoutRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx, async {
            state.auth_client.client().logout(req).await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn change_password(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resource_id = req.user_id.clone();
    let request_id = trace_ctx.request_id.clone();
    TRACE_CTX
        .scope(trace_ctx, async {
            state.auth_client.client().change_password(req).await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "change_password",
        resource: "user",
        resource_id: &resource_id,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

// ── User queries ────────────────────────────────────────────────────────────

pub async fn get_user(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(user_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .auth_client
                .client()
                .get_user(GetUserRequest { user_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_user_by_email(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(email): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .auth_client
                .client()
                .get_user_by_email(GetUserByEmailRequest { email })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

#[derive(Debug, Deserialize)]
pub struct ListUsersQuery {
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub per_page: u32,
    #[serde(default)]
    pub search: String,
    pub is_active: Option<bool>,
    pub is_locked: Option<bool>,
}

pub async fn list_users(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Query(q): Query<ListUsersQuery>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let req = ListUsersRequest {
        page: q.page,
        per_page: q.per_page,
        search: q.search,
        is_active: q.is_active.unwrap_or(false),
        has_is_active: q.is_active.is_some(),
        is_locked: q.is_locked.unwrap_or(false),
        has_is_locked: q.is_locked.is_some(),
    };
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.auth_client.client().list_users(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

// ── User administration ─────────────────────────────────────────────────────

pub async fn change_password_admin(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(user_id): Path<String>,
    Json(body): Json<ChangePasswordAdminRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resource_id = user_id.clone();
    let request_id = trace_ctx.request_id.clone();
    let req = ChangePasswordAdminRequest {
        user_id,
        new_password: body.new_password,
    };
    TRACE_CTX
        .scope(trace_ctx, async {
            state
                .auth_client
                .client()
                .change_password_admin(req)
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "change_password_admin",
        resource: "user",
        resource_id: &resource_id,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn forgot_password(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<ForgotPasswordRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx, async {
            state.auth_client.client().forgot_password(req).await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn reset_password(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    TRACE_CTX
        .scope(trace_ctx, async {
            state.auth_client.client().reset_password(req).await
        })
        .await
        .into_problem()?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn activate_user(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(user_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resource_id = user_id.clone();
    let request_id = trace_ctx.request_id.clone();
    TRACE_CTX
        .scope(trace_ctx, async {
            state
                .auth_client
                .client()
                .activate_user(ActivateUserRequest { user_id })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "activate_user",
        resource: "user",
        resource_id: &resource_id,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn deactivate_user(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(user_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resource_id = user_id.clone();
    let request_id = trace_ctx.request_id.clone();
    TRACE_CTX
        .scope(trace_ctx, async {
            state
                .auth_client
                .client()
                .deactivate_user(DeactivateUserRequest { user_id })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "deactivate_user",
        resource: "user",
        resource_id: &resource_id,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

// ── Roles ───────────────────────────────────────────────────────────────────

pub async fn list_roles(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.auth_client.client().list_roles(Empty {}).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn create_role(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Json(req): Json<CreateRoleRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resource_id = req.name.clone();
    let request_id = trace_ctx.request_id.clone();
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.auth_client.client().create_role(req).await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "create_role",
        resource: "role",
        resource_id: &resource_id,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &request_id,
        detail: None,
    });
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn list_user_roles(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(user_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .auth_client
                .client()
                .list_user_roles(ListUserRolesRequest { user_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn assign_role(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(user_id): Path<String>,
    Json(body): Json<AssignRoleRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let request_id = trace_ctx.request_id.clone();
    let resource_id = format!("{}/role/{}", user_id, body.role_id);
    let req = AssignRoleRequest {
        user_id,
        role_id: body.role_id,
        assigned_by: body.assigned_by,
    };
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.auth_client.client().assign_role(req).await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "assign_role",
        resource: "user",
        resource_id: &resource_id,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &request_id,
        detail: None,
    });
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn remove_user_role(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(user_role_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resource_id = user_role_id.clone();
    let request_id = trace_ctx.request_id.clone();
    TRACE_CTX
        .scope(trace_ctx, async {
            state
                .auth_client
                .client()
                .remove_user_role(RemoveUserRoleRequest { user_role_id })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "remove_user_role",
        resource: "user_role",
        resource_id: &resource_id,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

// ── Role permissions ────────────────────────────────────────────────────────

pub async fn add_role_permission(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path(role_id): Path<String>,
    Json(body): Json<AddRolePermissionRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let request_id = trace_ctx.request_id.clone();
    let resource_id = format!("{}/perm/{}", role_id, body.permission);
    let req = AddRolePermissionRequest {
        role_id,
        permission: body.permission,
    };
    TRACE_CTX
        .scope(trace_ctx, async {
            state.auth_client.client().add_role_permission(req).await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "add_role_permission",
        resource: "role",
        resource_id: &resource_id,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_role_permission(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    claims: Option<Extension<ddd_shared_kernel::jwt::StandardClaims>>,
    Path((role_id, permission)): Path<(String, String)>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let request_id = trace_ctx.request_id.clone();
    let resource_id = format!("{role_id}/perm/{permission}");
    TRACE_CTX
        .scope(trace_ctx, async {
            state
                .auth_client
                .client()
                .remove_role_permission(RemoveRolePermissionRequest {
                    role_id,
                    permission,
                })
                .await
        })
        .await
        .into_problem()?;
    audit(AuditEvent {
        action: "remove_role_permission",
        resource: "role",
        resource_id: &resource_id,
        actor: actor_from_ext(&claims.map(|e| e.0)),
        client_ip: &client_ip,
        request_id: &request_id,
        detail: None,
    });
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_role_permissions_by_id(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(role_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .auth_client
                .client()
                .get_role_permissions_by_id(GetRolePermissionsByIdRequest { role_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn check_permission(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<CheckPermissionRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.auth_client.client().check_permission(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_role_permissions(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<GetRolePermissionsRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.auth_client.client().get_role_permissions(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}
