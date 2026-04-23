use std::sync::Arc;

use async_trait::async_trait;
use ddd_application::{CommandHandler, QueryHandler, register_command_handler, register_query_handler};
use ddd_shared_kernel::{AppError, AppResult};

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::queries::*;
use crate::domain::entities::Department;
use crate::domain::ids::DepartmentId;
use crate::domain::repositories::DepartmentRepository;

// ── CreateDepartment ──────────────────────────────────────────────────────────

pub struct CreateDepartmentHandler {
    repo: Arc<dyn DepartmentRepository>,
}

#[async_trait]
impl CommandHandler<CreateDepartment> for CreateDepartmentHandler {
    async fn handle(&self, cmd: CreateDepartment) -> AppResult<Department> {
        if let Some(ref code) = cmd.department_code {
            if self.repo.code_exists(code).await? {
                return Err(AppError::conflict("A department with this code already exists"));
            }
        }
        let dept = Department::create(
            cmd.department_name,
            cmd.department_code,
            cmd.parent_department_id,
            cmd.head_of_department_id,
        );
        self.repo.save(&dept).await?;
        Ok(dept)
    }
}

register_command_handler!(CreateDepartment, AppDeps, |d: &AppDeps| {
    CreateDepartmentHandler { repo: d.department_repo.clone() }
});

// ── UpdateDepartment ──────────────────────────────────────────────────────────

pub struct UpdateDepartmentHandler {
    repo: Arc<dyn DepartmentRepository>,
}

#[async_trait]
impl CommandHandler<UpdateDepartment> for UpdateDepartmentHandler {
    async fn handle(&self, cmd: UpdateDepartment) -> AppResult<Department> {
        let mut dept = self.repo.find_by_id(DepartmentId::from_uuid(cmd.id)).await?
            .ok_or_else(|| AppError::not_found("Department", cmd.id.to_string()))?;
        dept.department_name = cmd.department_name;
        dept.department_code = cmd.department_code;
        dept.updated_at      = chrono::Utc::now();
        self.repo.save(&dept).await?;
        Ok(dept)
    }
}

register_command_handler!(UpdateDepartment, AppDeps, |d: &AppDeps| {
    UpdateDepartmentHandler { repo: d.department_repo.clone() }
});

// ── GetDepartment ─────────────────────────────────────────────────────────────

pub struct GetDepartmentHandler {
    repo: Arc<dyn DepartmentRepository>,
}

#[async_trait]
impl QueryHandler<GetDepartment> for GetDepartmentHandler {
    async fn handle(&self, q: GetDepartment) -> AppResult<Option<Department>> {
        self.repo.find_by_id(DepartmentId::from_uuid(q.id)).await
    }
}

register_query_handler!(GetDepartment, AppDeps, |d: &AppDeps| {
    GetDepartmentHandler { repo: d.department_repo.clone() }
});

// ── ListDepartments ───────────────────────────────────────────────────────────

pub struct ListDepartmentsHandler {
    repo: Arc<dyn DepartmentRepository>,
}

#[async_trait]
impl QueryHandler<ListDepartments> for ListDepartmentsHandler {
    async fn handle(&self, _q: ListDepartments) -> AppResult<Vec<Department>> {
        self.repo.get_all().await
    }
}

register_query_handler!(ListDepartments, AppDeps, |d: &AppDeps| {
    ListDepartmentsHandler { repo: d.department_repo.clone() }
});
