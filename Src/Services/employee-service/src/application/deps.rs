use std::sync::Arc;
use ddd_shared_kernel::storage::BlobStorage;
use crate::domain::repositories::{DepartmentRepository, DesignationRepository, EmployeeRepository};

pub struct AppDeps {
    pub employee_repo:    Arc<dyn EmployeeRepository>,
    pub department_repo:  Arc<dyn DepartmentRepository>,
    pub designation_repo: Arc<dyn DesignationRepository>,
    pub blob_storage:     Arc<dyn BlobStorage>,
    pub blob_bucket:      String,
    pub presign_ttl_secs: u64,
}
