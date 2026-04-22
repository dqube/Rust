//! Explicit business rules used by the command handlers.
//!
//! Keeping these rules in their own module makes them discoverable and
//! reusable: any handler that needs the same invariant calls the same
//! function rather than re-deriving it via ad-hoc repository queries.
//!
//! Each rule returns `AppResult<()>` — `Ok(())` when the rule holds,
//! and an [`AppError`] (`Conflict`, `NotFound`, or `BusinessRule`) when
//! it is violated. Callers bubble these errors up so the gRPC adapter
//! can map them onto the right `tonic::Status`.

use std::sync::Arc;

use ddd_shared_kernel::{AppError, AppResult};

use crate::domain::ids::{RoleId, UserId};
use crate::domain::repositories::{
    RolePermissionRepository, RoleRepository, UserRepository, UserRoleRepository,
};

// ── Uniqueness rules ────────────────────────────────────────────────────────

/// Email must not already be in use. Checked on `Register`.
pub async fn email_must_not_exist(
    email: &str,
    user_repo: &Arc<dyn UserRepository>,
) -> AppResult<()> {
    if user_repo.email_exists(email).await? {
        return Err(AppError::conflict(format!(
            "email {email} is already registered"
        )));
    }
    Ok(())
}

/// Username must not already be taken. Checked on `Register`.
pub async fn username_must_not_exist(
    username: &str,
    user_repo: &Arc<dyn UserRepository>,
) -> AppResult<()> {
    if user_repo.username_exists(username).await? {
        return Err(AppError::conflict(format!(
            "username {username} is already taken"
        )));
    }
    Ok(())
}

/// Role name must be unique across all roles. Checked on `CreateRole`.
pub async fn role_name_must_not_exist(
    name: &str,
    role_repo: &Arc<dyn RoleRepository>,
) -> AppResult<()> {
    if role_repo.role_name_exists(name).await? {
        return Err(AppError::conflict(format!("role {name} already exists")));
    }
    Ok(())
}

// ── Existence rules ─────────────────────────────────────────────────────────

/// A user with the given id must exist.
pub async fn user_must_exist(
    user_id: UserId,
    user_repo: &Arc<dyn UserRepository>,
) -> AppResult<()> {
    if user_repo.find_by_id(user_id).await?.is_none() {
        return Err(AppError::not_found("User", user_id.to_string()));
    }
    Ok(())
}

/// A role with the given id must exist.
pub async fn role_must_exist(
    role_id: RoleId,
    role_repo: &Arc<dyn RoleRepository>,
) -> AppResult<()> {
    if role_repo.find_by_id(role_id).await?.is_none() {
        return Err(AppError::not_found("Role", role_id.to_string()));
    }
    Ok(())
}

// ── Relationship rules ──────────────────────────────────────────────────────

/// A user cannot be assigned the same role twice. Checked on `AssignRole`.
pub async fn user_cannot_have_duplicate_role(
    user_id: UserId,
    role_id: RoleId,
    user_role_repo: &Arc<dyn UserRoleRepository>,
) -> AppResult<()> {
    if user_role_repo.user_has_role(user_id, role_id).await? {
        return Err(AppError::conflict("user already has this role"));
    }
    Ok(())
}

/// A given permission must not already be attached to a role.
/// Checked on `AddRolePermission` so the call is a clean no-op rather than
/// a silent duplicate-insert.
pub async fn permission_must_not_already_exist(
    role_id: RoleId,
    permission: &str,
    perm_repo: &Arc<dyn RolePermissionRepository>,
) -> AppResult<()> {
    if perm_repo.permission_exists(role_id, permission).await? {
        return Err(AppError::conflict(
            "permission is already assigned to this role",
        ));
    }
    Ok(())
}

// ── Password rules ──────────────────────────────────────────────────────────

/// Minimum password length enforced by the service.
pub const MIN_PASSWORD_LENGTH: usize = 6;

/// Enforce baseline password hygiene. Stays on the application layer because
/// the constant may eventually come from config (rotate per environment).
pub fn password_must_be_valid(password: &str) -> AppResult<()> {
    if password.len() < MIN_PASSWORD_LENGTH {
        return Err(AppError::validation(
            "password",
            format!("must be at least {MIN_PASSWORD_LENGTH} characters"),
        ));
    }
    Ok(())
}
