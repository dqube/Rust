use chrono::NaiveDate;
use ddd_application::impl_command;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::entities::{Department, Designation, Employee};
use crate::domain::enums::{EmploymentType, Gender};

// ── Employee ──────────────────────────────────────────────────────────────────

pub struct CreateEmployee {
    pub user_id:             Uuid,
    pub first_name:          String,
    pub last_name:           String,
    pub middle_name:         Option<String>,
    pub date_of_birth:       Option<NaiveDate>,
    pub gender:              Option<Gender>,
    pub email:               String,
    pub personal_email:      Option<String>,
    pub phone:               Option<String>,
    pub mobile:              Option<String>,
    pub department_id:       Option<Uuid>,
    pub designation_id:      Option<Uuid>,
    pub manager_id:          Option<Uuid>,
    pub employment_type:     EmploymentType,
    pub date_of_joining:     NaiveDate,
    pub salary:              Option<Decimal>,
    pub bank_account_number: Option<String>,
    pub bank_ifsc_code:      Option<String>,
    pub bank_name:           Option<String>,
}
impl_command!(CreateEmployee, Employee);

pub struct UpdateEmployee {
    pub id:                  Uuid,
    pub first_name:          String,
    pub last_name:           String,
    pub middle_name:         Option<String>,
    pub date_of_birth:       Option<NaiveDate>,
    pub gender:              Option<Gender>,
    pub email:               String,
    pub personal_email:      Option<String>,
    pub phone:               Option<String>,
    pub mobile:              Option<String>,
    pub department_id:       Option<Uuid>,
    pub designation_id:      Option<Uuid>,
    pub manager_id:          Option<Uuid>,
    pub employment_type:     EmploymentType,
    pub salary:              Option<Decimal>,
    pub bank_account_number: Option<String>,
    pub bank_ifsc_code:      Option<String>,
    pub bank_name:           Option<String>,
}
impl_command!(UpdateEmployee, Employee);

pub struct TerminateEmployee {
    pub id:              Uuid,
    pub date_of_leaving: NaiveDate,
}
impl_command!(TerminateEmployee, Employee);

pub struct ReactivateEmployee {
    pub id: Uuid,
}
impl_command!(ReactivateEmployee, Employee);

pub struct AssignEmployeeToStore {
    pub id:       Uuid,
    pub store_id: i32,
}
impl_command!(AssignEmployeeToStore, Employee);

pub struct RequestAvatarUploadUrl {
    pub employee_id:  Uuid,
    pub content_type: String,
}
impl_command!(RequestAvatarUploadUrl, (String, String, String));

pub struct ConfirmAvatarUpload {
    pub employee_id: Uuid,
    pub object_name: String,
}
impl_command!(ConfirmAvatarUpload, Employee);

pub struct DeleteAvatar {
    pub employee_id: Uuid,
}
impl_command!(DeleteAvatar, Employee);

// ── Department ────────────────────────────────────────────────────────────────

pub struct CreateDepartment {
    pub department_name:       String,
    pub department_code:       Option<String>,
    pub parent_department_id:  Option<Uuid>,
    pub head_of_department_id: Option<Uuid>,
}
impl_command!(CreateDepartment, Department);

pub struct UpdateDepartment {
    pub id:              Uuid,
    pub department_name: String,
    pub department_code: Option<String>,
}
impl_command!(UpdateDepartment, Department);

// ── Designation ───────────────────────────────────────────────────────────────

pub struct CreateDesignation {
    pub designation_name: String,
    pub level:            Option<i32>,
}
impl_command!(CreateDesignation, Designation);

pub struct UpdateDesignation {
    pub id:               Uuid,
    pub designation_name: String,
    pub level:            Option<i32>,
}
impl_command!(UpdateDesignation, Designation);
