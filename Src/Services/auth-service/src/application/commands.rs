//! Write-side command definitions.
//!
//! All return types flow through the mediator; see `handlers.rs` for behavior.
//! Business-rule enforcement lives in `crate::application::rules`.

use crate::domain::enums::UserType;
use crate::domain::ids::{RoleId, UserId, UserRoleId};
use crate::domain::token_service::IssuedTokens;

// ── Register ────────────────────────────────────────────────────────────────

pub struct Register {
    pub username: String,
    pub email: String,
    pub password: String,
    pub user_type: UserType,
    pub phone: Option<String>,
}
ddd_application::impl_command!(Register, UserId);

// ── Login ───────────────────────────────────────────────────────────────────

pub struct LoginResult {
    pub tokens: IssuedTokens,
    pub user_id: UserId,
    pub username: String,
    pub email: String,
    pub email_confirmed: bool,
    pub user_type: UserType,
    pub two_factor_enabled: bool,
    pub is_active: bool,
    pub is_locked: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct Login {
    pub username_or_email: String,
    pub password: String,
    pub ip_address: Option<String>,
}
ddd_application::impl_command!(Login, LoginResult);

// ── RefreshToken ────────────────────────────────────────────────────────────

pub struct RefreshToken {
    pub refresh_token: String,
    pub ip_address: Option<String>,
}
ddd_application::impl_command!(RefreshToken, LoginResult);

// ── Logout ──────────────────────────────────────────────────────────────────

pub struct Logout {
    pub refresh_token: String,
}
ddd_application::impl_command!(Logout, ());

// ── ChangePassword ──────────────────────────────────────────────────────────

pub struct ChangePassword {
    pub user_id: UserId,
    pub current_password: String,
    pub new_password: String,
}
ddd_application::impl_command!(ChangePassword, ());

// ── ChangePasswordAdmin ─────────────────────────────────────────────────────

pub struct ChangePasswordAdmin {
    pub user_id: UserId,
    pub new_password: String,
}
ddd_application::impl_command!(ChangePasswordAdmin, ());

// ── ForgotPassword ──────────────────────────────────────────────────────────

pub struct ForgotPassword {
    pub email: String,
}
ddd_application::impl_command!(ForgotPassword, ());

// ── ResetPassword ───────────────────────────────────────────────────────────

pub struct ResetPassword {
    pub token: String,
    pub new_password: String,
}
ddd_application::impl_command!(ResetPassword, ());

// ── Activate / Deactivate ──────────────────────────────────────────────────

pub struct ActivateUser {
    pub user_id: UserId,
}
ddd_application::impl_command!(ActivateUser, ());

pub struct DeactivateUser {
    pub user_id: UserId,
}
ddd_application::impl_command!(DeactivateUser, ());

// ── Roles ───────────────────────────────────────────────────────────────────

pub struct CreateRole {
    pub name: String,
    pub description: Option<String>,
}
ddd_application::impl_command!(CreateRole, RoleId);

pub struct AssignRole {
    pub user_id: UserId,
    pub role_id: RoleId,
    pub assigned_by: Option<UserId>,
}
ddd_application::impl_command!(AssignRole, UserRoleId);

pub struct RemoveUserRole {
    pub user_role_id: UserRoleId,
}
ddd_application::impl_command!(RemoveUserRole, ());

// ── RolePermissions ─────────────────────────────────────────────────────────

pub struct AddRolePermission {
    pub role_id: RoleId,
    pub permission: String,
}
ddd_application::impl_command!(AddRolePermission, ());

pub struct RemoveRolePermission {
    pub role_id: RoleId,
    pub permission: String,
}
ddd_application::impl_command!(RemoveRolePermission, ());

