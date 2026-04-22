use std::sync::Arc;

use ddd_api::grpc::ToGrpcStatus;
use ddd_application::Mediator;
use ddd_shared_kernel::PageRequest;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::application::commands::*;
use crate::application::queries::*;
use crate::domain::entities::{Department, Designation, Employee};
use crate::domain::enums::{EmploymentType, Gender};
use crate::proto::employee_service_server::{EmployeeService, EmployeeServiceServer};
use crate::proto::*;

pub struct EmployeeGrpcService {
    mediator: Arc<Mediator>,
}

impl EmployeeGrpcService {
    pub fn new(mediator: Arc<Mediator>) -> Self {
        Self { mediator }
    }

    pub fn into_server(self) -> EmployeeServiceServer<Self> {
        EmployeeServiceServer::new(self)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_id(s: &str, label: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("invalid {label}: {s}")))
}

fn parse_opt_id(s: &str) -> Option<Uuid> {
    if s.is_empty() { None } else { Uuid::parse_str(s).ok() }
}

fn parse_opt_date(s: &str) -> Option<chrono::NaiveDate> {
    if s.is_empty() { return None; }
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

fn parse_employment_type(s: &str) -> EmploymentType {
    use std::str::FromStr;
    EmploymentType::from_str(s).unwrap_or(EmploymentType::FullTime)
}

fn parse_opt_gender(s: &str) -> Option<Gender> {
    if s.is_empty() { return None; }
    use std::str::FromStr;
    Gender::from_str(s).ok()
}

fn parse_opt_decimal(s: &str) -> Option<rust_decimal::Decimal> {
    if s.is_empty() { return None; }
    s.parse().ok()
}

fn to_employee_message(e: Employee) -> EmployeeMessage {
    EmployeeMessage {
        id:                  e.id.to_string(),
        user_id:             e.user_id.to_string(),
        employee_code:       e.employee_code,
        first_name:          e.first_name,
        last_name:           e.last_name,
        middle_name:         e.middle_name.unwrap_or_default(),
        date_of_birth:       e.date_of_birth.map(|d| d.to_string()).unwrap_or_default(),
        gender:              e.gender.map(|g| g.to_string()).unwrap_or_default(),
        email:               e.email,
        personal_email:      e.personal_email.unwrap_or_default(),
        phone:               e.phone.unwrap_or_default(),
        mobile:              e.mobile.unwrap_or_default(),
        department_id:       e.department_id.map(|u| u.to_string()).unwrap_or_default(),
        designation_id:      e.designation_id.map(|u| u.to_string()).unwrap_or_default(),
        manager_id:          e.manager_id.map(|u| u.to_string()).unwrap_or_default(),
        employment_type:     e.employment_type.to_string(),
        date_of_joining:     e.date_of_joining.to_string(),
        date_of_leaving:     e.date_of_leaving.map(|d| d.to_string()).unwrap_or_default(),
        status:              e.status.to_string(),
        salary:              e.salary.map(|d| d.to_string()).unwrap_or_default(),
        bank_account_number: e.bank_account_number.unwrap_or_default(),
        bank_ifsc_code:      e.bank_ifsc_code.unwrap_or_default(),
        bank_name:           e.bank_name.unwrap_or_default(),
        avatar_object_name:  e.avatar_object_name.unwrap_or_default(),
        current_store_id:    e.current_store_id.unwrap_or(0),
        created_at:          e.created_at.to_rfc3339(),
        updated_at:          e.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
    }
}

fn to_department_message(d: Department) -> DepartmentMessage {
    DepartmentMessage {
        id:                    d.id.to_string(),
        department_name:       d.department_name,
        department_code:       d.department_code.unwrap_or_default(),
        parent_department_id:  d.parent_department_id.map(|u| u.to_string()).unwrap_or_default(),
        head_of_department_id: d.head_of_department_id.map(|u| u.to_string()).unwrap_or_default(),
        is_active:             d.is_active,
        created_at:            d.created_at.to_rfc3339(),
        updated_at:            d.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
    }
}

fn to_designation_message(d: Designation) -> DesignationMessage {
    DesignationMessage {
        id:               d.id.to_string(),
        designation_name: d.designation_name,
        level:            d.level.unwrap_or(0),
        is_active:        d.is_active,
        created_at:       d.created_at.to_rfc3339(),
        updated_at:       d.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
    }
}

// ── gRPC trait impl ───────────────────────────────────────────────────────────

#[tonic::async_trait]
impl EmployeeService for EmployeeGrpcService {
    // ── Employees ─────────────────────────────────────────────────────────────

    async fn create_employee(
        &self,
        req: Request<CreateEmployeeRequest>,
    ) -> Result<Response<EmployeeMessage>, Status> {
        let r = req.into_inner();
        let date_of_joining = chrono::NaiveDate::parse_from_str(&r.date_of_joining, "%Y-%m-%d")
            .map_err(|_| Status::invalid_argument("invalid date_of_joining; expected YYYY-MM-DD"))?;
        let cmd = CreateEmployee {
            user_id:             parse_id(&r.user_id, "user_id")?,
            first_name:          r.first_name,
            last_name:           r.last_name,
            middle_name:         if r.middle_name.is_empty() { None } else { Some(r.middle_name) },
            date_of_birth:       parse_opt_date(&r.date_of_birth),
            gender:              parse_opt_gender(&r.gender),
            email:               r.email,
            personal_email:      if r.personal_email.is_empty() { None } else { Some(r.personal_email) },
            phone:               if r.phone.is_empty() { None } else { Some(r.phone) },
            mobile:              if r.mobile.is_empty() { None } else { Some(r.mobile) },
            department_id:       parse_opt_id(&r.department_id),
            designation_id:      parse_opt_id(&r.designation_id),
            manager_id:          parse_opt_id(&r.manager_id),
            employment_type:     parse_employment_type(&r.employment_type),
            date_of_joining,
            salary:              parse_opt_decimal(&r.salary),
            bank_account_number: if r.bank_account_number.is_empty() { None } else { Some(r.bank_account_number) },
            bank_ifsc_code:      if r.bank_ifsc_code.is_empty() { None } else { Some(r.bank_ifsc_code) },
            bank_name:           if r.bank_name.is_empty() { None } else { Some(r.bank_name) },
        };
        let emp = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_employee_message(emp)))
    }

    async fn get_employee(
        &self,
        req: Request<GetEmployeeRequest>,
    ) -> Result<Response<EmployeeMessage>, Status> {
        let id = parse_id(&req.into_inner().id, "employee_id")?;
        let emp = self.mediator.query(GetEmployee { id }).await.map_err(|e| e.to_grpc_status())?
            .ok_or_else(|| Status::not_found(format!("Employee {id} not found")))?;
        Ok(Response::new(to_employee_message(emp)))
    }

    async fn get_employee_by_user_id(
        &self,
        req: Request<GetEmployeeByUserIdRequest>,
    ) -> Result<Response<EmployeeMessage>, Status> {
        let user_id = parse_id(&req.into_inner().user_id, "user_id")?;
        let emp = self.mediator.query(GetEmployeeByUserId { user_id }).await.map_err(|e| e.to_grpc_status())?
            .ok_or_else(|| Status::not_found(format!("No employee for user {user_id}")))?;
        Ok(Response::new(to_employee_message(emp)))
    }

    async fn get_employee_by_code(
        &self,
        req: Request<GetEmployeeByCodeRequest>,
    ) -> Result<Response<EmployeeMessage>, Status> {
        let code = req.into_inner().code;
        let emp = self.mediator.query(GetEmployeeByCode { code: code.clone() }).await.map_err(|e| e.to_grpc_status())?
            .ok_or_else(|| Status::not_found(format!("Employee code {code} not found")))?;
        Ok(Response::new(to_employee_message(emp)))
    }

    async fn update_employee(
        &self,
        req: Request<UpdateEmployeeRequest>,
    ) -> Result<Response<EmployeeMessage>, Status> {
        let r = req.into_inner();
        let cmd = UpdateEmployee {
            id:                  parse_id(&r.id, "employee_id")?,
            first_name:          r.first_name,
            last_name:           r.last_name,
            middle_name:         if r.middle_name.is_empty() { None } else { Some(r.middle_name) },
            date_of_birth:       parse_opt_date(&r.date_of_birth),
            gender:              parse_opt_gender(&r.gender),
            email:               r.email,
            personal_email:      if r.personal_email.is_empty() { None } else { Some(r.personal_email) },
            phone:               if r.phone.is_empty() { None } else { Some(r.phone) },
            mobile:              if r.mobile.is_empty() { None } else { Some(r.mobile) },
            department_id:       parse_opt_id(&r.department_id),
            designation_id:      parse_opt_id(&r.designation_id),
            manager_id:          parse_opt_id(&r.manager_id),
            employment_type:     parse_employment_type(&r.employment_type),
            salary:              parse_opt_decimal(&r.salary),
            bank_account_number: if r.bank_account_number.is_empty() { None } else { Some(r.bank_account_number) },
            bank_ifsc_code:      if r.bank_ifsc_code.is_empty() { None } else { Some(r.bank_ifsc_code) },
            bank_name:           if r.bank_name.is_empty() { None } else { Some(r.bank_name) },
        };
        let emp = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_employee_message(emp)))
    }

    async fn terminate_employee(
        &self,
        req: Request<TerminateEmployeeRequest>,
    ) -> Result<Response<EmployeeMessage>, Status> {
        let r = req.into_inner();
        let date_of_leaving = chrono::NaiveDate::parse_from_str(&r.date_of_leaving, "%Y-%m-%d")
            .map_err(|_| Status::invalid_argument("invalid date_of_leaving; expected YYYY-MM-DD"))?;
        let cmd = TerminateEmployee {
            id: parse_id(&r.id, "employee_id")?,
            date_of_leaving,
        };
        let emp = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_employee_message(emp)))
    }

    async fn reactivate_employee(
        &self,
        req: Request<ReactivateEmployeeRequest>,
    ) -> Result<Response<EmployeeMessage>, Status> {
        let id = parse_id(&req.into_inner().id, "employee_id")?;
        let emp = self.mediator.send(ReactivateEmployee { id }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_employee_message(emp)))
    }

    async fn list_employees(
        &self,
        req: Request<ListEmployeesRequest>,
    ) -> Result<Response<ListEmployeesResponse>, Status> {
        let r = req.into_inner();
        let page     = if r.page == 0 { 1 } else { r.page };
        let per_page = if r.per_page == 0 { 20 } else { r.per_page };
        let q = ListEmployees {
            status_filter: if r.status_filter.is_empty() { None } else { Some(r.status_filter) },
            department_id: parse_opt_id(&r.department_id),
            search:        if r.search.is_empty() { None } else { Some(r.search) },
            req:           PageRequest { page, per_page },
        };
        let page_result = self.mediator.query(q).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ListEmployeesResponse {
            items:       page_result.items.into_iter().map(to_employee_message).collect(),
            total:       page_result.total,
            page:        page_result.page,
            per_page:    page_result.per_page,
            total_pages: page_result.total_pages,
        }))
    }

    async fn assign_employee_to_store(
        &self,
        req: Request<AssignEmployeeToStoreRequest>,
    ) -> Result<Response<EmployeeMessage>, Status> {
        let r = req.into_inner();
        let cmd = AssignEmployeeToStore {
            id:       parse_id(&r.id, "employee_id")?,
            store_id: r.store_id,
        };
        let emp = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_employee_message(emp)))
    }

    // ── Avatar ────────────────────────────────────────────────────────────────

    async fn request_avatar_upload_url(
        &self,
        req: Request<RequestAvatarUploadUrlRequest>,
    ) -> Result<Response<RequestAvatarUploadUrlResponse>, Status> {
        let r = req.into_inner();
        let cmd = RequestAvatarUploadUrl {
            employee_id:  parse_id(&r.employee_id, "employee_id")?,
            content_type: r.content_type,
        };
        let (upload_url, object_name, expires_at) = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(RequestAvatarUploadUrlResponse { upload_url, object_name, expires_at }))
    }

    async fn confirm_avatar_upload(
        &self,
        req: Request<ConfirmAvatarUploadRequest>,
    ) -> Result<Response<EmployeeMessage>, Status> {
        let r = req.into_inner();
        let cmd = ConfirmAvatarUpload {
            employee_id: parse_id(&r.employee_id, "employee_id")?,
            object_name: r.object_name,
        };
        let emp = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_employee_message(emp)))
    }

    async fn delete_avatar(
        &self,
        req: Request<DeleteAvatarRequest>,
    ) -> Result<Response<EmployeeMessage>, Status> {
        let employee_id = parse_id(&req.into_inner().employee_id, "employee_id")?;
        let emp = self.mediator.send(DeleteAvatar { employee_id }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_employee_message(emp)))
    }

    async fn get_avatar_url(
        &self,
        req: Request<GetAvatarUrlRequest>,
    ) -> Result<Response<GetAvatarUrlResponse>, Status> {
        let employee_id = parse_id(&req.into_inner().employee_id, "employee_id")?;
        let (url, expires_at) = self.mediator.query(GetAvatarUrl { employee_id }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetAvatarUrlResponse { url, expires_at }))
    }

    // ── Departments ───────────────────────────────────────────────────────────

    async fn list_departments(
        &self,
        _req: Request<ListDepartmentsRequest>,
    ) -> Result<Response<ListDepartmentsResponse>, Status> {
        let depts = self.mediator.query(ListDepartments {}).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ListDepartmentsResponse {
            items: depts.into_iter().map(to_department_message).collect(),
        }))
    }

    async fn create_department(
        &self,
        req: Request<CreateDepartmentRequest>,
    ) -> Result<Response<DepartmentMessage>, Status> {
        let r = req.into_inner();
        let cmd = CreateDepartment {
            department_name:       r.department_name,
            department_code:       if r.department_code.is_empty() { None } else { Some(r.department_code) },
            parent_department_id:  parse_opt_id(&r.parent_department_id),
            head_of_department_id: parse_opt_id(&r.head_of_department_id),
        };
        let dept = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_department_message(dept)))
    }

    async fn get_department(
        &self,
        req: Request<GetDepartmentRequest>,
    ) -> Result<Response<DepartmentMessage>, Status> {
        let id = parse_id(&req.into_inner().id, "department_id")?;
        let dept = self.mediator.query(GetDepartment { id }).await.map_err(|e| e.to_grpc_status())?
            .ok_or_else(|| Status::not_found(format!("Department {id} not found")))?;
        Ok(Response::new(to_department_message(dept)))
    }

    async fn update_department(
        &self,
        req: Request<UpdateDepartmentRequest>,
    ) -> Result<Response<DepartmentMessage>, Status> {
        let r = req.into_inner();
        let cmd = UpdateDepartment {
            id:               parse_id(&r.id, "department_id")?,
            department_name:  r.department_name,
            department_code:  if r.department_code.is_empty() { None } else { Some(r.department_code) },
        };
        let dept = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_department_message(dept)))
    }

    // ── Designations ──────────────────────────────────────────────────────────

    async fn list_designations(
        &self,
        _req: Request<ListDesignationsRequest>,
    ) -> Result<Response<ListDesignationsResponse>, Status> {
        let desigs = self.mediator.query(ListDesignations {}).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ListDesignationsResponse {
            items: desigs.into_iter().map(to_designation_message).collect(),
        }))
    }

    async fn create_designation(
        &self,
        req: Request<CreateDesignationRequest>,
    ) -> Result<Response<DesignationMessage>, Status> {
        let r = req.into_inner();
        let cmd = CreateDesignation {
            designation_name: r.designation_name,
            level:            if r.level == 0 { None } else { Some(r.level) },
        };
        let desig = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_designation_message(desig)))
    }

    async fn get_designation(
        &self,
        req: Request<GetDesignationRequest>,
    ) -> Result<Response<DesignationMessage>, Status> {
        let id = parse_id(&req.into_inner().id, "designation_id")?;
        let desig = self.mediator.query(GetDesignation { id }).await.map_err(|e| e.to_grpc_status())?
            .ok_or_else(|| Status::not_found(format!("Designation {id} not found")))?;
        Ok(Response::new(to_designation_message(desig)))
    }

    async fn update_designation(
        &self,
        req: Request<UpdateDesignationRequest>,
    ) -> Result<Response<DesignationMessage>, Status> {
        let r = req.into_inner();
        let cmd = UpdateDesignation {
            id:               parse_id(&r.id, "designation_id")?,
            designation_name: r.designation_name,
            level:            if r.level == 0 { None } else { Some(r.level) },
        };
        let desig = self.mediator.send(cmd).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_designation_message(desig)))
    }
}
