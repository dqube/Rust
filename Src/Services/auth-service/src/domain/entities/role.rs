use chrono::Utc;

use ddd_domain::{define_aggregate, impl_aggregate, impl_aggregate_events};
use ddd_shared_kernel::{AppError, AppResult};

use crate::domain::enums::RoleType;
use crate::domain::ids::RoleId;

// ── Role ──────────────────────────────────────────────────────────────────────

define_aggregate!(Role, RoleId, {
    pub name:        String,
    pub role_type:   RoleType,
    pub description: Option<String>,
    pub is_active:   bool,
});

impl_aggregate!(Role, RoleId);
impl_aggregate_events!(Role);

impl Clone for Role {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            version: self.version,
            created_at: self.created_at,
            updated_at: self.updated_at,
            domain_events: Vec::new(),
            name: self.name.clone(),
            role_type: self.role_type,
            description: self.description.clone(),
            is_active: self.is_active,
        }
    }
}

impl Role {
    pub fn create_custom(name: String, description: Option<String>) -> AppResult<Self> {
        let name = name.trim().to_owned();
        if name.is_empty() {
            return Err(AppError::validation("name", "must not be empty"));
        }
        if name.len() > 100 {
            return Err(AppError::validation("name", "must be at most 100 characters"));
        }
        let now = Utc::now();
        Ok(Self {
            id: RoleId::new(),
            version: 0,
            domain_events: Vec::new(),
            created_at: now,
            updated_at: now,
            name,
            role_type: RoleType::Custom,
            description,
            is_active: true,
        })
    }

    pub fn builtin(id: RoleId, name: &str, description: &str) -> Self {
        let now = Utc::now();
        Self {
            id,
            version: 0,
            domain_events: Vec::new(),
            created_at: now,
            updated_at: now,
            name: name.to_owned(),
            role_type: RoleType::BuiltIn,
            description: Some(description.to_owned()),
            is_active: true,
        }
    }
}

// ── RolePermission ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RolePermission {
    pub role_id:    RoleId,
    pub permission: String,
    pub created_at: chrono::DateTime<Utc>,
}

impl RolePermission {
    pub fn new(role_id: RoleId, permission: String) -> AppResult<Self> {
        let permission = permission.trim().to_owned();
        if permission.is_empty() {
            return Err(AppError::validation("permission", "must not be empty"));
        }
        Ok(Self {
            role_id,
            permission,
            created_at: Utc::now(),
        })
    }
}
