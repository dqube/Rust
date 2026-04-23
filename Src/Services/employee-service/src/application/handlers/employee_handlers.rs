use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use ddd_application::{CommandHandler, QueryHandler, register_command_handler, register_query_handler};
use ddd_shared_kernel::{AppError, AppResult, BlobStorage, Page};

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::queries::*;
use crate::domain::entities::Employee;
use crate::domain::ids::EmployeeId;
use crate::domain::repositories::EmployeeRepository;

// ── CreateEmployee ────────────────────────────────────────────────────────────

pub struct CreateEmployeeHandler {
    repo: Arc<dyn EmployeeRepository>,
}

#[async_trait]
impl CommandHandler<CreateEmployee> for CreateEmployeeHandler {
    async fn handle(&self, cmd: CreateEmployee) -> AppResult<Employee> {
        if self.repo.user_id_exists(cmd.user_id).await? {
            return Err(AppError::conflict("An employee already exists for this user"));
        }
        if self.repo.email_exists(&cmd.email).await? {
            return Err(AppError::conflict("An employee with this email already exists"));
        }
        let emp = Employee::create(
            cmd.user_id, cmd.first_name, cmd.last_name, cmd.middle_name,
            cmd.date_of_birth, cmd.gender, cmd.email, cmd.personal_email,
            cmd.phone, cmd.mobile, cmd.department_id, cmd.designation_id,
            cmd.manager_id, cmd.employment_type, cmd.date_of_joining,
            cmd.salary, cmd.bank_account_number, cmd.bank_ifsc_code, cmd.bank_name,
        );
        self.repo.save(&emp).await?;
        Ok(emp)
    }
}

register_command_handler!(CreateEmployee, AppDeps, |d: &AppDeps| {
    CreateEmployeeHandler { repo: d.employee_repo.clone() }
});

// ── UpdateEmployee ────────────────────────────────────────────────────────────

pub struct UpdateEmployeeHandler {
    repo: Arc<dyn EmployeeRepository>,
}

#[async_trait]
impl CommandHandler<UpdateEmployee> for UpdateEmployeeHandler {
    async fn handle(&self, cmd: UpdateEmployee) -> AppResult<Employee> {
        let mut emp = self.repo.find_by_id(EmployeeId::from_uuid(cmd.id)).await?
            .ok_or_else(|| AppError::not_found("Employee", cmd.id.to_string()))?;
        emp.update(
            cmd.first_name, cmd.last_name, cmd.middle_name,
            cmd.date_of_birth, cmd.gender, cmd.email, cmd.personal_email,
            cmd.phone, cmd.mobile, cmd.department_id, cmd.designation_id,
            cmd.manager_id, cmd.employment_type, cmd.salary,
            cmd.bank_account_number, cmd.bank_ifsc_code, cmd.bank_name,
        );
        self.repo.save(&emp).await?;
        Ok(emp)
    }
}

register_command_handler!(UpdateEmployee, AppDeps, |d: &AppDeps| {
    UpdateEmployeeHandler { repo: d.employee_repo.clone() }
});

// ── TerminateEmployee ─────────────────────────────────────────────────────────

pub struct TerminateEmployeeHandler {
    repo: Arc<dyn EmployeeRepository>,
}

#[async_trait]
impl CommandHandler<TerminateEmployee> for TerminateEmployeeHandler {
    async fn handle(&self, cmd: TerminateEmployee) -> AppResult<Employee> {
        let mut emp = self.repo.find_by_id(EmployeeId::from_uuid(cmd.id)).await?
            .ok_or_else(|| AppError::not_found("Employee", cmd.id.to_string()))?;
        emp.terminate(cmd.date_of_leaving);
        self.repo.save(&emp).await?;
        Ok(emp)
    }
}

register_command_handler!(TerminateEmployee, AppDeps, |d: &AppDeps| {
    TerminateEmployeeHandler { repo: d.employee_repo.clone() }
});

// ── ReactivateEmployee ────────────────────────────────────────────────────────

pub struct ReactivateEmployeeHandler {
    repo: Arc<dyn EmployeeRepository>,
}

#[async_trait]
impl CommandHandler<ReactivateEmployee> for ReactivateEmployeeHandler {
    async fn handle(&self, cmd: ReactivateEmployee) -> AppResult<Employee> {
        let mut emp = self.repo.find_by_id(EmployeeId::from_uuid(cmd.id)).await?
            .ok_or_else(|| AppError::not_found("Employee", cmd.id.to_string()))?;
        emp.reactivate();
        self.repo.save(&emp).await?;
        Ok(emp)
    }
}

register_command_handler!(ReactivateEmployee, AppDeps, |d: &AppDeps| {
    ReactivateEmployeeHandler { repo: d.employee_repo.clone() }
});

// ── AssignEmployeeToStore ─────────────────────────────────────────────────────

pub struct AssignEmployeeToStoreHandler {
    repo: Arc<dyn EmployeeRepository>,
}

#[async_trait]
impl CommandHandler<AssignEmployeeToStore> for AssignEmployeeToStoreHandler {
    async fn handle(&self, cmd: AssignEmployeeToStore) -> AppResult<Employee> {
        let mut emp = self.repo.find_by_id(EmployeeId::from_uuid(cmd.id)).await?
            .ok_or_else(|| AppError::not_found("Employee", cmd.id.to_string()))?;
        emp.assign_store(cmd.store_id);
        self.repo.save(&emp).await?;
        Ok(emp)
    }
}

register_command_handler!(AssignEmployeeToStore, AppDeps, |d: &AppDeps| {
    AssignEmployeeToStoreHandler { repo: d.employee_repo.clone() }
});

// ── RequestAvatarUploadUrl ────────────────────────────────────────────────────

pub struct RequestAvatarUploadUrlHandler {
    repo:             Arc<dyn EmployeeRepository>,
    blob_storage:     Arc<dyn BlobStorage>,
    blob_bucket:      String,
    presign_ttl_secs: u64,
}

#[async_trait]
impl CommandHandler<RequestAvatarUploadUrl> for RequestAvatarUploadUrlHandler {
    async fn handle(&self, cmd: RequestAvatarUploadUrl) -> AppResult<(String, String, String)> {
        self.repo.find_by_id(EmployeeId::from_uuid(cmd.employee_id)).await?
            .ok_or_else(|| AppError::not_found("Employee", cmd.employee_id.to_string()))?;
        let key = format!("employees/{}/avatar/{}", cmd.employee_id, uuid::Uuid::new_v4());
        let presigned = self.blob_storage
            .presigned_put(&self.blob_bucket, &key, &cmd.content_type, Duration::from_secs(self.presign_ttl_secs))
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        Ok((presigned.url, key, presigned.expires_at.to_rfc3339()))
    }
}

register_command_handler!(RequestAvatarUploadUrl, AppDeps, |d: &AppDeps| {
    RequestAvatarUploadUrlHandler {
        repo:             d.employee_repo.clone(),
        blob_storage:     d.blob_storage.clone(),
        blob_bucket:      d.blob_bucket.clone(),
        presign_ttl_secs: d.presign_ttl_secs,
    }
});

// ── ConfirmAvatarUpload ───────────────────────────────────────────────────────

pub struct ConfirmAvatarUploadHandler {
    repo: Arc<dyn EmployeeRepository>,
}

#[async_trait]
impl CommandHandler<ConfirmAvatarUpload> for ConfirmAvatarUploadHandler {
    async fn handle(&self, cmd: ConfirmAvatarUpload) -> AppResult<Employee> {
        let mut emp = self.repo.find_by_id(EmployeeId::from_uuid(cmd.employee_id)).await?
            .ok_or_else(|| AppError::not_found("Employee", cmd.employee_id.to_string()))?;
        emp.set_avatar(cmd.object_name);
        self.repo.save(&emp).await?;
        Ok(emp)
    }
}

register_command_handler!(ConfirmAvatarUpload, AppDeps, |d: &AppDeps| {
    ConfirmAvatarUploadHandler { repo: d.employee_repo.clone() }
});

// ── DeleteAvatar ──────────────────────────────────────────────────────────────

pub struct DeleteAvatarHandler {
    repo: Arc<dyn EmployeeRepository>,
}

#[async_trait]
impl CommandHandler<DeleteAvatar> for DeleteAvatarHandler {
    async fn handle(&self, cmd: DeleteAvatar) -> AppResult<Employee> {
        let mut emp = self.repo.find_by_id(EmployeeId::from_uuid(cmd.employee_id)).await?
            .ok_or_else(|| AppError::not_found("Employee", cmd.employee_id.to_string()))?;
        emp.clear_avatar();
        self.repo.save(&emp).await?;
        Ok(emp)
    }
}

register_command_handler!(DeleteAvatar, AppDeps, |d: &AppDeps| {
    DeleteAvatarHandler { repo: d.employee_repo.clone() }
});

// ── GetEmployee ───────────────────────────────────────────────────────────────

pub struct GetEmployeeHandler {
    repo: Arc<dyn EmployeeRepository>,
}

#[async_trait]
impl QueryHandler<GetEmployee> for GetEmployeeHandler {
    async fn handle(&self, q: GetEmployee) -> AppResult<Option<Employee>> {
        self.repo.find_by_id(EmployeeId::from_uuid(q.id)).await
    }
}

register_query_handler!(GetEmployee, AppDeps, |d: &AppDeps| {
    GetEmployeeHandler { repo: d.employee_repo.clone() }
});

// ── GetEmployeeByUserId ───────────────────────────────────────────────────────

pub struct GetEmployeeByUserIdHandler {
    repo: Arc<dyn EmployeeRepository>,
}

#[async_trait]
impl QueryHandler<GetEmployeeByUserId> for GetEmployeeByUserIdHandler {
    async fn handle(&self, q: GetEmployeeByUserId) -> AppResult<Option<Employee>> {
        self.repo.find_by_user_id(q.user_id).await
    }
}

register_query_handler!(GetEmployeeByUserId, AppDeps, |d: &AppDeps| {
    GetEmployeeByUserIdHandler { repo: d.employee_repo.clone() }
});

// ── GetEmployeeByCode ─────────────────────────────────────────────────────────

pub struct GetEmployeeByCodeHandler {
    repo: Arc<dyn EmployeeRepository>,
}

#[async_trait]
impl QueryHandler<GetEmployeeByCode> for GetEmployeeByCodeHandler {
    async fn handle(&self, q: GetEmployeeByCode) -> AppResult<Option<Employee>> {
        self.repo.find_by_code(&q.code).await
    }
}

register_query_handler!(GetEmployeeByCode, AppDeps, |d: &AppDeps| {
    GetEmployeeByCodeHandler { repo: d.employee_repo.clone() }
});

// ── ListEmployees ─────────────────────────────────────────────────────────────

pub struct ListEmployeesHandler {
    repo: Arc<dyn EmployeeRepository>,
}

#[async_trait]
impl QueryHandler<ListEmployees> for ListEmployeesHandler {
    async fn handle(&self, q: ListEmployees) -> AppResult<Page<Employee>> {
        self.repo.list_paged(
            q.status_filter.as_deref(),
            q.department_id,
            q.search.as_deref(),
            &q.req,
        ).await
    }
}

register_query_handler!(ListEmployees, AppDeps, |d: &AppDeps| {
    ListEmployeesHandler { repo: d.employee_repo.clone() }
});

// ── GetAvatarUrl ──────────────────────────────────────────────────────────────

pub struct GetAvatarUrlHandler {
    repo:             Arc<dyn EmployeeRepository>,
    blob_storage:     Arc<dyn BlobStorage>,
    blob_bucket:      String,
    presign_ttl_secs: u64,
}

#[async_trait]
impl QueryHandler<GetAvatarUrl> for GetAvatarUrlHandler {
    async fn handle(&self, q: GetAvatarUrl) -> AppResult<(String, String)> {
        let emp = self.repo.find_by_id(EmployeeId::from_uuid(q.employee_id)).await?
            .ok_or_else(|| AppError::not_found("Employee", q.employee_id.to_string()))?;
        let key = emp.avatar_object_name
            .ok_or_else(|| AppError::not_found("Avatar", q.employee_id.to_string()))?;
        let presigned = self.blob_storage
            .presigned_get(&self.blob_bucket, &key, Duration::from_secs(self.presign_ttl_secs))
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        Ok((presigned.url, presigned.expires_at.to_rfc3339()))
    }
}

register_query_handler!(GetAvatarUrl, AppDeps, |d: &AppDeps| {
    GetAvatarUrlHandler {
        repo:             d.employee_repo.clone(),
        blob_storage:     d.blob_storage.clone(),
        blob_bucket:      d.blob_bucket.clone(),
        presign_ttl_secs: d.presign_ttl_secs,
    }
});
