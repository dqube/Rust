use uuid::Uuid;

use ddd_domain::{define_aggregate, impl_aggregate, impl_aggregate_events};

use crate::domain::ids::DepartmentId;

// ── Department ────────────────────────────────────────────────────────────────

define_aggregate!(Department, DepartmentId, {
    pub department_name:       String,
    pub department_code:       Option<String>,
    pub parent_department_id:  Option<Uuid>,
    pub head_of_department_id: Option<Uuid>,
    pub is_active:             bool,
});

impl_aggregate!(Department, DepartmentId);
impl_aggregate_events!(Department);

impl Department {
    pub fn create(
        department_name:       String,
        department_code:       Option<String>,
        parent_department_id:  Option<Uuid>,
        head_of_department_id: Option<Uuid>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: DepartmentId::new(),
            version: 0,
            domain_events: Vec::new(),
            created_at: now,
            updated_at: now,
            department_name,
            department_code,
            parent_department_id,
            head_of_department_id,
            is_active: true,
        }
    }
}
