use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use super::enums::{EmployeeStatus, EmploymentType, Gender};

// ── Employee ─────────────────────────────────────────────────────────────────

pub struct Employee {
    pub id:                  Uuid,
    pub user_id:             Uuid,
    pub employee_code:       String,
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
    pub date_of_leaving:     Option<NaiveDate>,
    pub status:              EmployeeStatus,
    pub salary:              Option<Decimal>,
    pub bank_account_number: Option<String>,
    pub bank_ifsc_code:      Option<String>,
    pub bank_name:           Option<String>,
    pub avatar_object_name:  Option<String>,
    pub current_store_id:    Option<i32>,
    pub created_at:          chrono::DateTime<Utc>,
    pub updated_at:          Option<chrono::DateTime<Utc>>,
}

impl Employee {
    /// Generate a deterministic employee code: EMP-<timestamp_millis>.
    fn generate_code() -> String {
        format!("EMP-{}", Utc::now().timestamp_millis())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create(
        user_id: Uuid,
        first_name: String,
        last_name: String,
        middle_name: Option<String>,
        date_of_birth: Option<NaiveDate>,
        gender: Option<Gender>,
        email: String,
        personal_email: Option<String>,
        phone: Option<String>,
        mobile: Option<String>,
        department_id: Option<Uuid>,
        designation_id: Option<Uuid>,
        manager_id: Option<Uuid>,
        employment_type: EmploymentType,
        date_of_joining: NaiveDate,
        salary: Option<Decimal>,
        bank_account_number: Option<String>,
        bank_ifsc_code: Option<String>,
        bank_name: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            employee_code: Self::generate_code(),
            first_name,
            last_name,
            middle_name,
            date_of_birth,
            gender,
            email,
            personal_email,
            phone,
            mobile,
            department_id,
            designation_id,
            manager_id,
            employment_type,
            date_of_joining,
            date_of_leaving: None,
            status: EmployeeStatus::Active,
            salary,
            bank_account_number,
            bank_ifsc_code,
            bank_name,
            avatar_object_name: None,
            current_store_id: None,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        first_name: String,
        last_name: String,
        middle_name: Option<String>,
        date_of_birth: Option<NaiveDate>,
        gender: Option<Gender>,
        email: String,
        personal_email: Option<String>,
        phone: Option<String>,
        mobile: Option<String>,
        department_id: Option<Uuid>,
        designation_id: Option<Uuid>,
        manager_id: Option<Uuid>,
        employment_type: EmploymentType,
        salary: Option<Decimal>,
        bank_account_number: Option<String>,
        bank_ifsc_code: Option<String>,
        bank_name: Option<String>,
    ) {
        self.first_name          = first_name;
        self.last_name           = last_name;
        self.middle_name         = middle_name;
        self.date_of_birth       = date_of_birth;
        self.gender              = gender;
        self.email               = email;
        self.personal_email      = personal_email;
        self.phone               = phone;
        self.mobile              = mobile;
        self.department_id       = department_id;
        self.designation_id      = designation_id;
        self.manager_id          = manager_id;
        self.employment_type     = employment_type;
        self.salary              = salary;
        self.bank_account_number = bank_account_number;
        self.bank_ifsc_code      = bank_ifsc_code;
        self.bank_name           = bank_name;
        self.updated_at          = Some(Utc::now());
    }

    pub fn terminate(&mut self, date_of_leaving: NaiveDate) {
        self.status         = EmployeeStatus::Terminated;
        self.date_of_leaving = Some(date_of_leaving);
        self.updated_at     = Some(Utc::now());
    }

    pub fn reactivate(&mut self) {
        self.status         = EmployeeStatus::Active;
        self.date_of_leaving = None;
        self.updated_at     = Some(Utc::now());
    }

    pub fn assign_store(&mut self, store_id: i32) {
        self.current_store_id = Some(store_id);
        self.updated_at       = Some(Utc::now());
    }

    pub fn set_avatar(&mut self, object_name: String) {
        self.avatar_object_name = Some(object_name);
        self.updated_at         = Some(Utc::now());
    }

    pub fn clear_avatar(&mut self) {
        self.avatar_object_name = None;
        self.updated_at         = Some(Utc::now());
    }
}

// ── Department ───────────────────────────────────────────────────────────────

pub struct Department {
    pub id:                    Uuid,
    pub department_name:       String,
    pub department_code:       Option<String>,
    pub parent_department_id:  Option<Uuid>,
    pub head_of_department_id: Option<Uuid>,
    pub is_active:             bool,
    pub created_at:            chrono::DateTime<Utc>,
    pub updated_at:            Option<chrono::DateTime<Utc>>,
}

impl Department {
    pub fn create(
        department_name: String,
        department_code: Option<String>,
        parent_department_id: Option<Uuid>,
        head_of_department_id: Option<Uuid>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            department_name,
            department_code,
            parent_department_id,
            head_of_department_id,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
        }
    }
}

// ── Designation ──────────────────────────────────────────────────────────────

pub struct Designation {
    pub id:               Uuid,
    pub designation_name: String,
    pub level:            Option<i32>,
    pub is_active:        bool,
    pub created_at:       chrono::DateTime<Utc>,
    pub updated_at:       Option<chrono::DateTime<Utc>>,
}

impl Designation {
    pub fn create(designation_name: String, level: Option<i32>) -> Self {
        Self {
            id: Uuid::new_v4(),
            designation_name,
            level,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
        }
    }
}
