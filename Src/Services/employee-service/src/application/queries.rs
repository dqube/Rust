use ddd_application::impl_query;
use ddd_shared_kernel::{Page, PageRequest};
use uuid::Uuid;

use crate::domain::entities::{Department, Designation, Employee};

// ── Employee ──────────────────────────────────────────────────────────────────

pub struct GetEmployee {
    pub id: Uuid,
}
impl_query!(GetEmployee, Option<Employee>);

pub struct GetEmployeeByUserId {
    pub user_id: Uuid,
}
impl_query!(GetEmployeeByUserId, Option<Employee>);

pub struct GetEmployeeByCode {
    pub code: String,
}
impl_query!(GetEmployeeByCode, Option<Employee>);

pub struct ListEmployees {
    pub status_filter: Option<String>,
    pub department_id: Option<Uuid>,
    pub search:        Option<String>,
    pub req:           PageRequest,
}
impl_query!(ListEmployees, Page<Employee>);

pub struct GetAvatarUrl {
    pub employee_id: Uuid,
}
impl_query!(GetAvatarUrl, (String, String));

// ── Department ────────────────────────────────────────────────────────────────

pub struct GetDepartment {
    pub id: Uuid,
}
impl_query!(GetDepartment, Option<Department>);

pub struct ListDepartments {}
impl_query!(ListDepartments, Vec<Department>);

// ── Designation ───────────────────────────────────────────────────────────────

pub struct GetDesignation {
    pub id: Uuid,
}
impl_query!(GetDesignation, Option<Designation>);

pub struct ListDesignations {}
impl_query!(ListDesignations, Vec<Designation>);
