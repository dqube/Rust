use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ddd_shared_kernel::{AppError, AppResult, Page, PageRequest};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};
use uuid::Uuid;

use crate::domain::entities::{Department, Designation, Employee};
use crate::domain::enums::{EmployeeStatus, EmploymentType, Gender};
use crate::domain::ids::{DepartmentId, DesignationId, EmployeeId};
use crate::domain::repositories::{DepartmentRepository, DesignationRepository, EmployeeRepository};
use crate::infrastructure::db::models::{department, designation, employee};

// ── helpers ───────────────────────────────────────────────────────────────────

fn db_err(e: sea_orm::DbErr) -> AppError {
    AppError::internal(e.to_string())
}

fn to_utc(dt: sea_orm::prelude::DateTimeWithTimeZone) -> DateTime<Utc> {
    dt.with_timezone(&Utc)
}
fn opt_to_utc(dt: Option<sea_orm::prelude::DateTimeWithTimeZone>) -> Option<DateTime<Utc>> {
    dt.map(to_utc)
}
fn from_utc(dt: DateTime<Utc>) -> sea_orm::prelude::DateTimeWithTimeZone {
    dt.fixed_offset()
}

// ── model → domain mappers ────────────────────────────────────────────────────

fn m2employee(m: employee::Model) -> Employee {
    Employee {
        id:                  EmployeeId::from_uuid(m.id),
        version:             0,
        created_at:          to_utc(m.created_at),
        updated_at:          opt_to_utc(m.updated_at).unwrap_or_else(|| to_utc(m.created_at)),
        domain_events:       Vec::new(),
        user_id:             m.user_id,
        employee_code:       m.employee_code,
        first_name:          m.first_name,
        last_name:           m.last_name,
        middle_name:         m.middle_name,
        date_of_birth:       m.date_of_birth,
        gender:              m.gender.as_deref().and_then(|g| Gender::from_str(g).ok()),
        email:               m.email,
        personal_email:      m.personal_email,
        phone:               m.phone,
        mobile:              m.mobile,
        department_id:       m.department_id,
        designation_id:      m.designation_id,
        manager_id:          m.manager_id,
        employment_type:     m.employment_type.as_deref()
            .and_then(|t| EmploymentType::from_str(t).ok())
            .unwrap_or(EmploymentType::FullTime),
        date_of_joining:     m.date_of_joining,
        date_of_leaving:     m.date_of_leaving,
        status:              EmployeeStatus::from_str(&m.status).unwrap_or(EmployeeStatus::Active),
        salary:              m.salary,
        bank_account_number: m.bank_account_number,
        bank_ifsc_code:      m.bank_ifsc_code,
        bank_name:           m.bank_name,
        avatar_object_name:  m.avatar_object_name,
        current_store_id:    m.current_store_id,
    }
}

fn m2department(m: department::Model) -> Department {
    Department {
        id:                    DepartmentId::from_uuid(m.id),
        version:               0,
        created_at:            to_utc(m.created_at),
        updated_at:            opt_to_utc(m.updated_at).unwrap_or_else(|| to_utc(m.created_at)),
        domain_events:         Vec::new(),
        department_name:       m.department_name,
        department_code:       m.department_code,
        parent_department_id:  m.parent_department_id,
        head_of_department_id: m.head_of_department_id,
        is_active:             m.is_active,
    }
}

fn m2designation(m: designation::Model) -> Designation {
    Designation {
        id:               DesignationId::from_uuid(m.id),
        version:          0,
        created_at:       to_utc(m.created_at),
        updated_at:       opt_to_utc(m.updated_at).unwrap_or_else(|| to_utc(m.created_at)),
        domain_events:    Vec::new(),
        designation_name: m.designation_name,
        level:            m.level,
        is_active:        m.is_active,
    }
}

// ── PgEmployeeRepository ──────────────────────────────────────────────────────

pub struct PgEmployeeRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl EmployeeRepository for PgEmployeeRepository {
    async fn find_by_id(&self, id: EmployeeId) -> AppResult<Option<Employee>> {
        Ok(employee::Entity::find_by_id(id.as_uuid())
            .one(&*self.0).await.map_err(db_err)?
            .map(m2employee))
    }

    async fn find_by_user_id(&self, user_id: Uuid) -> AppResult<Option<Employee>> {
        Ok(employee::Entity::find()
            .filter(employee::Column::UserId.eq(user_id))
            .one(&*self.0).await.map_err(db_err)?
            .map(m2employee))
    }

    async fn find_by_code(&self, code: &str) -> AppResult<Option<Employee>> {
        Ok(employee::Entity::find()
            .filter(employee::Column::EmployeeCode.eq(code.to_string()))
            .one(&*self.0).await.map_err(db_err)?
            .map(m2employee))
    }

    async fn code_exists(&self, code: &str) -> AppResult<bool> {
        Ok(employee::Entity::find()
            .filter(employee::Column::EmployeeCode.eq(code.to_string()))
            .count(&*self.0).await.map_err(db_err)? > 0)
    }

    async fn user_id_exists(&self, user_id: Uuid) -> AppResult<bool> {
        Ok(employee::Entity::find()
            .filter(employee::Column::UserId.eq(user_id))
            .count(&*self.0).await.map_err(db_err)? > 0)
    }

    async fn email_exists(&self, email: &str) -> AppResult<bool> {
        Ok(employee::Entity::find()
            .filter(employee::Column::Email.eq(email.to_string()))
            .count(&*self.0).await.map_err(db_err)? > 0)
    }

    async fn list_paged(
        &self,
        status_filter: Option<&str>,
        department_id: Option<Uuid>,
        search: Option<&str>,
        req: &PageRequest,
    ) -> AppResult<Page<Employee>> {
        let mut q = employee::Entity::find();
        if let Some(s) = status_filter {
            if !s.is_empty() {
                q = q.filter(employee::Column::Status.eq(s.to_string()));
            }
        }
        if let Some(did) = department_id {
            q = q.filter(employee::Column::DepartmentId.eq(did));
        }
        if let Some(s) = search {
            use sea_orm::Condition;
            q = q.filter(
                Condition::any()
                    .add(employee::Column::FirstName.contains(s.to_lowercase()))
                    .add(employee::Column::LastName.contains(s.to_lowercase()))
                    .add(employee::Column::EmployeeCode.contains(s.to_lowercase()))
                    .add(employee::Column::Email.contains(s.to_lowercase())),
            );
        }
        let total = q.clone().count(&*self.0).await.map_err(db_err)? as u64;
        let page = req.page().max(1);
        let per_page = req.per_page().max(1);
        let offset = ((page - 1) * per_page) as u64;
        let items = q
            .order_by_asc(employee::Column::EmployeeCode)
            .offset(offset)
            .limit(per_page as u64)
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(m2employee).collect();
        Ok(Page::new(items, total, page, per_page))
    }

    async fn save(&self, e: &Employee) -> AppResult<()> {
        let active = employee::ActiveModel {
            id:                  Set(e.id.as_uuid()),
            user_id:             Set(e.user_id),
            employee_code:       Set(e.employee_code.clone()),
            first_name:          Set(e.first_name.clone()),
            last_name:           Set(e.last_name.clone()),
            middle_name:         Set(e.middle_name.clone()),
            date_of_birth:       Set(e.date_of_birth),
            gender:              Set(e.gender.as_ref().map(|g| g.to_string())),
            email:               Set(e.email.clone()),
            personal_email:      Set(e.personal_email.clone()),
            phone:               Set(e.phone.clone()),
            mobile:              Set(e.mobile.clone()),
            department_id:       Set(e.department_id),
            designation_id:      Set(e.designation_id),
            manager_id:          Set(e.manager_id),
            employment_type:     Set(Some(e.employment_type.to_string())),
            date_of_joining:     Set(e.date_of_joining),
            date_of_leaving:     Set(e.date_of_leaving),
            status:              Set(e.status.to_string()),
            salary:              Set(e.salary),
            bank_account_number: Set(e.bank_account_number.clone()),
            bank_ifsc_code:      Set(e.bank_ifsc_code.clone()),
            bank_name:           Set(e.bank_name.clone()),
            avatar_object_name:  Set(e.avatar_object_name.clone()),
            current_store_id:    Set(e.current_store_id),
            created_at:          Set(from_utc(e.created_at)),
            updated_at:          Set(Some(from_utc(e.updated_at))),
        };
        employee::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(employee::Column::Id)
                    .update_columns([
                        employee::Column::FirstName,
                        employee::Column::LastName,
                        employee::Column::MiddleName,
                        employee::Column::DateOfBirth,
                        employee::Column::Gender,
                        employee::Column::Email,
                        employee::Column::PersonalEmail,
                        employee::Column::Phone,
                        employee::Column::Mobile,
                        employee::Column::DepartmentId,
                        employee::Column::DesignationId,
                        employee::Column::ManagerId,
                        employee::Column::EmploymentType,
                        employee::Column::DateOfLeaving,
                        employee::Column::Status,
                        employee::Column::Salary,
                        employee::Column::BankAccountNumber,
                        employee::Column::BankIfscCode,
                        employee::Column::BankName,
                        employee::Column::AvatarObjectName,
                        employee::Column::CurrentStoreId,
                        employee::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
}

// ── PgDepartmentRepository ────────────────────────────────────────────────────

pub struct PgDepartmentRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl DepartmentRepository for PgDepartmentRepository {
    async fn find_by_id(&self, id: DepartmentId) -> AppResult<Option<Department>> {
        Ok(department::Entity::find_by_id(id.as_uuid())
            .one(&*self.0).await.map_err(db_err)?
            .map(m2department))
    }

    async fn code_exists(&self, code: &str) -> AppResult<bool> {
        Ok(department::Entity::find()
            .filter(department::Column::DepartmentCode.eq(code.to_string()))
            .count(&*self.0).await.map_err(db_err)? > 0)
    }

    async fn get_all(&self) -> Result<Vec<Department>, AppError> {
        Ok(department::Entity::find()
            .order_by_asc(department::Column::DepartmentName)
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(m2department).collect())
    }

    async fn save(&self, d: &Department) -> AppResult<()> {
        let active = department::ActiveModel {
            id:                    Set(d.id.as_uuid()),
            department_name:       Set(d.department_name.clone()),
            department_code:       Set(d.department_code.clone()),
            parent_department_id:  Set(d.parent_department_id),
            head_of_department_id: Set(d.head_of_department_id),
            is_active:             Set(d.is_active),
            created_at:            Set(from_utc(d.created_at)),
            updated_at:            Set(Some(from_utc(d.updated_at))),
        };
        department::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(department::Column::Id)
                    .update_columns([
                        department::Column::DepartmentName,
                        department::Column::DepartmentCode,
                        department::Column::ParentDepartmentId,
                        department::Column::HeadOfDepartmentId,
                        department::Column::IsActive,
                        department::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
}

// ── PgDesignationRepository ───────────────────────────────────────────────────

pub struct PgDesignationRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl DesignationRepository for PgDesignationRepository {
    async fn find_by_id(&self, id: DesignationId) -> AppResult<Option<Designation>> {
        Ok(designation::Entity::find_by_id(id.as_uuid())
            .one(&*self.0).await.map_err(db_err)?
            .map(m2designation))
    }

    async fn name_exists(&self, name: &str) -> AppResult<bool> {
        Ok(designation::Entity::find()
            .filter(designation::Column::DesignationName.eq(name.to_string()))
            .count(&*self.0).await.map_err(db_err)? > 0)
    }

    async fn get_all(&self) -> Result<Vec<Designation>, AppError> {
        Ok(designation::Entity::find()
            .order_by_asc(designation::Column::DesignationName)
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(m2designation).collect())
    }

    async fn save(&self, d: &Designation) -> AppResult<()> {
        let active = designation::ActiveModel {
            id:               Set(d.id.as_uuid()),
            designation_name: Set(d.designation_name.clone()),
            level:            Set(d.level),
            is_active:        Set(d.is_active),
            created_at:       Set(from_utc(d.created_at)),
            updated_at:       Set(Some(from_utc(d.updated_at))),
        };
        designation::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(designation::Column::Id)
                    .update_columns([
                        designation::Column::DesignationName,
                        designation::Column::Level,
                        designation::Column::IsActive,
                        designation::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
}
