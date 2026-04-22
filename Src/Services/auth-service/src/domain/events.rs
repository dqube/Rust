//! Domain + integration events.
//!
//! Domain events fire in-process via the mediator; integration events are
//! appended to the outbox so downstream services consume them reliably.

use chrono::{DateTime, Utc};
use ddd_shared_kernel::{DomainEvent, IntegrationEvent};
use serde::{Deserialize, Serialize};
use std::any::Any;

use super::enums::UserType;
use super::ids::{RoleId, UserId, UserRoleId};

// ── Helpers ─────────────────────────────────────────────────────────────────

macro_rules! domain_event {
    ($ty:ident, $name:literal) => {
        impl DomainEvent for $ty {
            fn event_name(&self) -> &'static str {
                $name
            }
            fn occurred_at(&self) -> DateTime<Utc> {
                self.occurred_at
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}

// ── Domain events ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRegistered {
    pub user_id: UserId,
    pub username: String,
    pub email: String,
    pub user_type: UserType,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(UserRegistered, "auth.user.registered");

impl IntegrationEvent for UserRegistered {
    fn event_type(&self) -> &'static str {
        "auth.user.registered.v1"
    }
    fn subject(&self) -> String {
        "auth.user.registered".to_owned()
    }
    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserActivated {
    pub user_id: UserId,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(UserActivated, "auth.user.activated");

impl IntegrationEvent for UserActivated {
    fn event_type(&self) -> &'static str {
        "auth.user.activated.v1"
    }
    fn subject(&self) -> String {
        "auth.user.activated".to_owned()
    }
    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDeactivated {
    pub user_id: UserId,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(UserDeactivated, "auth.user.deactivated");

impl IntegrationEvent for UserDeactivated {
    fn event_type(&self) -> &'static str {
        "auth.user.deactivated.v1"
    }
    fn subject(&self) -> String {
        "auth.user.deactivated".to_owned()
    }
    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPasswordChanged {
    pub user_id: UserId,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(UserPasswordChanged, "auth.user.password_changed");

impl IntegrationEvent for UserPasswordChanged {
    fn event_type(&self) -> &'static str {
        "auth.user.password_changed.v1"
    }
    fn subject(&self) -> String {
        "auth.user.password_changed".to_owned()
    }
    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRoleAssigned {
    pub user_role_id: UserRoleId,
    pub user_id: UserId,
    pub role_id: RoleId,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(UserRoleAssigned, "auth.user_role.assigned");

impl IntegrationEvent for UserRoleAssigned {
    fn event_type(&self) -> &'static str {
        "auth.user_role.assigned.v1"
    }
    fn subject(&self) -> String {
        "auth.user_role.assigned".to_owned()
    }
    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRoleRemoved {
    pub user_role_id: UserRoleId,
    pub user_id: UserId,
    pub role_id: RoleId,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(UserRoleRemoved, "auth.user_role.removed");

impl IntegrationEvent for UserRoleRemoved {
    fn event_type(&self) -> &'static str {
        "auth.user_role.removed.v1"
    }
    fn subject(&self) -> String {
        "auth.user_role.removed".to_owned()
    }
    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetRequested {
    pub user_id: UserId,
    pub email: String,
    pub reset_token: String,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(PasswordResetRequested, "auth.password.reset_requested");

impl IntegrationEvent for PasswordResetRequested {
    fn event_type(&self) -> &'static str {
        "auth.password.reset_requested.v1"
    }
    fn subject(&self) -> String {
        "auth.password.reset_requested".to_owned()
    }
    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
}
