use ddd_shared_kernel::Page;

use crate::domain::entities::{Role, User, UserRole};
use crate::domain::ids::{RoleId, UserId};

// ── GetUser ─────────────────────────────────────────────────────────────────

pub struct GetUser {
    pub user_id: UserId,
}
ddd_application::impl_query!(GetUser, Option<User>);

pub struct GetUserByEmail {
    pub email: String,
}
ddd_application::impl_query!(GetUserByEmail, Option<User>);

// ── ListUsers — pagination matches product-service ──────────────────────────

pub struct ListUsers {
    pub page: u32,
    pub per_page: u32,
    pub search: Option<String>,
    pub is_active: Option<bool>,
    pub is_locked: Option<bool>,
}
ddd_application::impl_query!(ListUsers, Page<User>);

// ── CheckPermission / GetRolePermissions ───────────────────────────────────

pub struct CheckPermission {
    pub user_id: UserId,
    pub permission: String,
}
ddd_application::impl_query!(CheckPermission, bool);

pub struct GetRolePermissionsByNames {
    pub role_names: Vec<String>,
}
ddd_application::impl_query!(GetRolePermissionsByNames, Vec<String>);

pub struct GetRolePermissionsById {
    pub role_id: RoleId,
}
ddd_application::impl_query!(GetRolePermissionsById, Vec<String>);

// ── Roles ───────────────────────────────────────────────────────────────────

pub struct ListRoles;
ddd_application::impl_query!(ListRoles, Vec<Role>);

pub struct ListUserRoles {
    pub user_id: UserId,
}
ddd_application::impl_query!(ListUserRoles, Vec<UserRoleWithName>);

/// Carry the role name alongside a UserRole row (saves a round-trip for
/// the admin UI which needs both).
#[derive(Debug, Clone)]
pub struct UserRoleWithName {
    pub user_role: UserRole,
    pub role_name: String,
    pub role_type: String,
}
