use ddd_application::impl_query;
use ddd_shared_kernel::{Page, PageRequest};
use uuid::Uuid;

use crate::domain::entities::{Department, Designation, Employee};

impl_query! { GetEmployee        { id: Uuid }              -> Option<Employee>       }
impl_query! { GetEmployeeByUserId { user_id: Uuid }        -> Option<Employee>       }
impl_query! { GetEmployeeByCode  { code: String }          -> Option<Employee>       }

impl_query! {
    ListEmployees {
        status_filter: Option<String>,
        department_id: Option<Uuid>,
        search:        Option<String>,
        req:           PageRequest,
    } -> Page<Employee>
}

impl_query! { GetDepartment      { id: Uuid }              -> Option<Department>     }
impl_query! { ListDepartments    {}                        -> Vec<Department>        }

impl_query! { GetDesignation     { id: Uuid }              -> Option<Designation>    }
impl_query! { ListDesignations   {}                        -> Vec<Designation>       }

impl_query! {
    GetAvatarUrl {
        employee_id: Uuid,
    } -> (String, String)  // (url, expires_at)
}
