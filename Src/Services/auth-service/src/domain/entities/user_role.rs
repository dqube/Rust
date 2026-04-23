use chrono::{DateTime, Utc};

use ddd_domain::{define_aggregate, impl_aggregate, impl_aggregate_events};
use ddd_shared_kernel::DomainEvent;

use crate::domain::events::{UserRoleAssigned, UserRoleRemoved};
use crate::domain::ids::{RoleId, UserId, UserRoleId};

// ── UserRole ──────────────────────────────────────────────────────────────────

define_aggregate!(UserRole, UserRoleId, {
    pub user_id:     UserId,
    pub role_id:     RoleId,
    pub assigned_by: Option<UserId>,
    pub expires_at:  Option<DateTime<Utc>>,
});

impl_aggregate!(UserRole, UserRoleId);
impl_aggregate_events!(UserRole);

impl Clone for UserRole {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            version: self.version,
            created_at: self.created_at,
            updated_at: self.updated_at,
            domain_events: Vec::new(),
            user_id: self.user_id,
            role_id: self.role_id,
            assigned_by: self.assigned_by,
            expires_at: self.expires_at,
        }
    }
}

impl UserRole {
    pub fn assign(
        user_id:     UserId,
        role_id:     RoleId,
        assigned_by: Option<UserId>,
        expires_at:  Option<DateTime<Utc>>,
    ) -> Self {
        let id = UserRoleId::new();
        let now = Utc::now();
        let mut ur = Self {
            id,
            version: 0,
            domain_events: Vec::new(),
            created_at: now,
            updated_at: now,
            user_id,
            role_id,
            assigned_by,
            expires_at,
        };
        ur.domain_events.push(Box::new(UserRoleAssigned {
            user_role_id: id,
            user_id,
            role_id,
            occurred_at: now,
        }));
        ur
    }

    pub fn emit_removed(&mut self) {
        self.domain_events.push(Box::new(UserRoleRemoved {
            user_role_id: self.id,
            user_id: self.user_id,
            role_id: self.role_id,
            occurred_at: Utc::now(),
        }));
    }

    pub fn drain_events(&mut self) -> Vec<Box<dyn DomainEvent>> {
        std::mem::take(&mut self.domain_events)
    }
}
