use async_trait::async_trait;
use ddd_shared_kernel::{AppError, AppResult, Page, PageRequest};
use uuid::Uuid;

use super::entities::{Department, Designation, Employee};
use super::ids::{DepartmentId, DesignationId, EmployeeId};

#[async_trait]
pub trait EmployeeRepository: Send + Sync {
    async fn find_by_id(&self, id: EmployeeId) -> AppResult<Option<Employee>>;
    async fn find_by_user_id(&self, user_id: Uuid) -> AppResult<Option<Employee>>;
    async fn find_by_code(&self, code: &str) -> AppResult<Option<Employee>>;
    async fn code_exists(&self, code: &str) -> AppResult<bool>;
    async fn user_id_exists(&self, user_id: Uuid) -> AppResult<bool>;
    async fn email_exists(&self, email: &str) -> AppResult<bool>;
    async fn list_paged(
        &self,
        status_filter: Option<&str>,
        department_id: Option<Uuid>,
        search: Option<&str>,
        req: &PageRequest,
    ) -> AppResult<Page<Employee>>;
    async fn save(&self, employee: &Employee) -> AppResult<()>;
}

#[async_trait]
pub trait DepartmentRepository: Send + Sync {
    async fn find_by_id(&self, id: DepartmentId) -> AppResult<Option<Department>>;
    async fn code_exists(&self, code: &str) -> AppResult<bool>;
    async fn get_all(&self) -> Result<Vec<Department>, AppError>;
    async fn save(&self, department: &Department) -> AppResult<()>;
}

#[async_trait]
pub trait DesignationRepository: Send + Sync {
    async fn find_by_id(&self, id: DesignationId) -> AppResult<Option<Designation>>;
    async fn name_exists(&self, name: &str) -> AppResult<bool>;
    async fn get_all(&self) -> Result<Vec<Designation>, AppError>;
    async fn save(&self, designation: &Designation) -> AppResult<()>;
}
