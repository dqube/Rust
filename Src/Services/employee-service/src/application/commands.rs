use chrono::NaiveDate;
use ddd_application::impl_command;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::entities::{Department, Designation, Employee};
use crate::domain::enums::{EmploymentType, Gender};

// ── Employee commands ─────────────────────────────────────────────────────────

impl_command! {
    CreateEmployee {
        user_id:             Uuid,
        first_name:          String,
        last_name:           String,
        middle_name:         Option<String>,
        date_of_birth:       Option<NaiveDate>,
        gender:              Option<Gender>,
        email:               String,
        personal_email:      Option<String>,
        phone:               Option<String>,
        mobile:              Option<String>,
        department_id:       Option<Uuid>,
        designation_id:      Option<Uuid>,
        manager_id:          Option<Uuid>,
        employment_type:     EmploymentType,
        date_of_joining:     NaiveDate,
        salary:              Option<Decimal>,
        bank_account_number: Option<String>,
        bank_ifsc_code:      Option<String>,
        bank_name:           Option<String>,
    } -> Employee
}

impl_command! {
    UpdateEmployee {
        id:                  Uuid,
        first_name:          String,
        last_name:           String,
        middle_name:         Option<String>,
        date_of_birth:       Option<NaiveDate>,
        gender:              Option<Gender>,
        email:               String,
        personal_email:      Option<String>,
        phone:               Option<String>,
        mobile:              Option<String>,
        department_id:       Option<Uuid>,
        designation_id:      Option<Uuid>,
        manager_id:          Option<Uuid>,
        employment_type:     EmploymentType,
        salary:              Option<Decimal>,
        bank_account_number: Option<String>,
        bank_ifsc_code:      Option<String>,
        bank_name:           Option<String>,
    } -> Employee
}

impl_command! {
    TerminateEmployee {
        id:              Uuid,
        date_of_leaving: NaiveDate,
    } -> Employee
}

impl_command! {
    ReactivateEmployee {
        id: Uuid,
    } -> Employee
}

impl_command! {
    AssignEmployeeToStore {
        id:       Uuid,
        store_id: i32,
    } -> Employee
}

impl_command! {
    RequestAvatarUploadUrl {
        employee_id:  Uuid,
        content_type: String,
    } -> (String, String, String)  // (upload_url, object_name, expires_at)
}

impl_command! {
    ConfirmAvatarUpload {
        employee_id: Uuid,
        object_name: String,
    } -> Employee
}

impl_command! {
    DeleteAvatar {
        employee_id: Uuid,
    } -> Employee
}

// ── Department commands ───────────────────────────────────────────────────────

impl_command! {
    CreateDepartment {
        department_name:       String,
        department_code:       Option<String>,
        parent_department_id:  Option<Uuid>,
        head_of_department_id: Option<Uuid>,
    } -> Department
}

impl_command! {
    UpdateDepartment {
        id:               Uuid,
        department_name:  String,
        department_code:  Option<String>,
    } -> Department
}

// ── Designation commands ──────────────────────────────────────────────────────

impl_command! {
    CreateDesignation {
        designation_name: String,
        level:            Option<i32>,
    } -> Designation
}

impl_command! {
    UpdateDesignation {
        id:               Uuid,
        designation_name: String,
        level:            Option<i32>,
    } -> Designation
}
