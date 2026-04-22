use async_trait::async_trait;
use ddd_shared_kernel::{AppError, Page, PageRequest};
use uuid::Uuid;

use super::entities::{Department, Designation, Employee};

#[async_trait]
pub trait EmployeeRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Employee>, AppError>;
    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Option<Employee>, AppError>;
    async fn find_by_code(&self, code: &str) -> Result<Option<Employee>, AppError>;
    async fn code_exists(&self, code: &str) -> Result<bool, AppError>;
    async fn user_id_exists(&self, user_id: Uuid) -> Result<bool, AppError>;
    async fn email_exists(&self, email: &str) -> Result<bool, AppError>;
    async fn list_paged(
        &self,
        status_filter: Option<&str>,
        department_id: Option<Uuid>,
        search: Option<&str>,
        req: &PageRequest,
    ) -> Result<Page<Employee>, AppError>;
    async fn save(&self, employee: &Employee) -> Result<(), AppError>;
}

#[async_trait]
pub trait DepartmentRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Department>, AppError>;
    async fn code_exists(&self, code: &str) -> Result<bool, AppError>;
    async fn get_all(&self) -> Result<Vec<Department>, AppError>;
    async fn save(&self, department: &Department) -> Result<(), AppError>;
}

#[async_trait]
pub trait DesignationRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Designation>, AppError>;
    async fn name_exists(&self, name: &str) -> Result<bool, AppError>;
    async fn get_all(&self) -> Result<Vec<Designation>, AppError>;
    async fn save(&self, designation: &Designation) -> Result<(), AppError>;
}
