use ddd_domain::{define_aggregate, impl_aggregate, impl_aggregate_events};

use crate::domain::ids::DesignationId;

// ── Designation ───────────────────────────────────────────────────────────────

define_aggregate!(Designation, DesignationId, {
    pub designation_name: String,
    pub level:            Option<i32>,
    pub is_active:        bool,
});

impl_aggregate!(Designation, DesignationId);
impl_aggregate_events!(Designation);

impl Designation {
    pub fn create(designation_name: String, level: Option<i32>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: DesignationId::new(),
            version: 0,
            domain_events: Vec::new(),
            created_at: now,
            updated_at: now,
            designation_name,
            level,
            is_active: true,
        }
    }
}
