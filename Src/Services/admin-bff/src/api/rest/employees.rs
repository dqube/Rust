//! Employee pass-through handlers — REST → gRPC proxy.

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use ddd_bff::prelude::*;

use crate::application::state::AppState;
use crate::proto_employee::{
    AssignEmployeeToStoreRequest, ConfirmAvatarUploadRequest, CreateDepartmentRequest,
    CreateDesignationRequest, CreateEmployeeRequest, DeleteAvatarRequest, GetAvatarUrlRequest,
    GetDepartmentRequest, GetDesignationRequest, GetEmployeeByCodeRequest,
    GetEmployeeByUserIdRequest, GetEmployeeRequest, ListDepartmentsRequest,
    ListDesignationsRequest, ListEmployeesRequest, ReactivateEmployeeRequest,
    RequestAvatarUploadUrlRequest, TerminateEmployeeRequest, UpdateDepartmentRequest,
    UpdateDesignationRequest, UpdateEmployeeRequest,
};

// ── Employees ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListEmployeesParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub status_filter: Option<String>,
    pub department_id: Option<String>,
    pub search: Option<String>,
}

pub async fn list_employees(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Query(params): Query<ListEmployeesParams>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
                .client()
                .list_employees(ListEmployeesRequest {
                    page: params.page.unwrap_or(1),
                    per_page: params.per_page.unwrap_or(20),
                    status_filter: params.status_filter.unwrap_or_default(),
                    department_id: params.department_id.unwrap_or_default(),
                    search: params.search.unwrap_or_default(),
                })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn create_employee(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<CreateEmployeeRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.employee_client.client().create_employee(req).await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn get_employee(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
                .client()
                .get_employee(GetEmployeeRequest { id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_employee_by_user_id(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(user_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
                .client()
                .get_employee_by_user_id(GetEmployeeByUserIdRequest { user_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_employee_by_code(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
                .client()
                .get_employee_by_code(GetEmployeeByCodeRequest { code })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn update_employee(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<UpdateEmployeeRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.employee_client.client().update_employee(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn terminate_employee(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<TerminateEmployeeRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state.employee_client.client().terminate_employee(req).await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn reactivate_employee(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
                .client()
                .reactivate_employee(ReactivateEmployeeRequest { id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn assign_to_store(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<AssignEmployeeToStoreRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
                .client()
                .assign_employee_to_store(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

// ── Avatar ────────────────────────────────────────────────────────────────────

pub async fn request_avatar_upload_url(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(employee_id): Path<String>,
    Json(mut req): Json<RequestAvatarUploadUrlRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.employee_id = employee_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
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
    Path(employee_id): Path<String>,
    Json(mut req): Json<ConfirmAvatarUploadRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.employee_id = employee_id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
                .client()
                .confirm_avatar_upload(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn delete_avatar(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(employee_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
                .client()
                .delete_avatar(DeleteAvatarRequest { employee_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn get_avatar_url(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(employee_id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
                .client()
                .get_avatar_url(GetAvatarUrlRequest { employee_id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

// ── Departments ───────────────────────────────────────────────────────────────

pub async fn list_departments(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
                .client()
                .list_departments(ListDepartmentsRequest {})
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn create_department(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<CreateDepartmentRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
                .client()
                .create_department(req)
                .await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn get_department(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
                .client()
                .get_department(GetDepartmentRequest { id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn update_department(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<UpdateDepartmentRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
                .client()
                .update_department(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

// ── Designations ──────────────────────────────────────────────────────────────

pub async fn list_designations(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
                .client()
                .list_designations(ListDesignationsRequest {})
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn create_designation(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Json(req): Json<CreateDesignationRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
                .client()
                .create_designation(req)
                .await
        })
        .await
        .into_problem()?;
    Ok((StatusCode::CREATED, Json(resp.into_inner())))
}

pub async fn get_designation(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ProblemDetail> {
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
                .client()
                .get_designation(GetDesignationRequest { id })
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}

pub async fn update_designation(
    State(state): State<AppState>,
    Extension(trace_ctx): Extension<RequestTraceContext>,
    Path(id): Path<String>,
    Json(mut req): Json<UpdateDesignationRequest>,
) -> Result<impl IntoResponse, ProblemDetail> {
    req.id = id;
    let resp = TRACE_CTX
        .scope(trace_ctx, async {
            state
                .employee_client
                .client()
                .update_designation(req)
                .await
        })
        .await
        .into_problem()?;
    Ok(Json(resp.into_inner()))
}
