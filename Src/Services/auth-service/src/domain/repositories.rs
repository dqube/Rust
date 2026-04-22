//! Repository ports for the auth aggregates.
//!
//! Pagination follows the workspace-standard [`Page`] / [`PageRequest`] shape
//! (same as product-service). Implementations live in `infrastructure::db`.

use async_trait::async_trait;
use ddd_shared_kernel::{AppResult, Page, PageRequest};

use super::entities::{PasswordResetToken, RefreshToken, Role, RolePermission, User, UserRole};
use super::ids::{PasswordResetTokenId, RefreshTokenId, RoleId, UserId, UserRoleId};

/// Filters applied to `UserRepository::list_paged`.
#[derive(Debug, Default, Clone)]
pub struct UserListFilter {
    /// Case-insensitive substring match against username or email.
    pub search: Option<String>,
    pub is_active: Option<bool>,
    pub is_locked: Option<bool>,
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: UserId) -> AppResult<Option<User>>;
    async fn find_by_username(&self, username: &str) -> AppResult<Option<User>>;
    async fn find_by_email(&self, email: &str) -> AppResult<Option<User>>;
    /// Accepts a username **or** email — checks both columns.
    async fn find_by_username_or_email(&self, value: &str) -> AppResult<Option<User>>;
    async fn email_exists(&self, email: &str) -> AppResult<bool>;
    async fn username_exists(&self, username: &str) -> AppResult<bool>;
    async fn list_paged(
        &self,
        page: PageRequest,
        filter: UserListFilter,
    ) -> AppResult<Page<User>>;
    async fn save(&self, user: &User) -> AppResult<()>;
}

#[async_trait]
pub trait RoleRepository: Send + Sync {
    async fn find_by_id(&self, id: RoleId) -> AppResult<Option<Role>>;
    async fn find_by_name(&self, name: &str) -> AppResult<Option<Role>>;
    async fn role_name_exists(&self, name: &str) -> AppResult<bool>;
    async fn list_all(&self) -> AppResult<Vec<Role>>;
    async fn save(&self, role: &Role) -> AppResult<()>;
}

#[async_trait]
pub trait UserRoleRepository: Send + Sync {
    async fn find_by_id(&self, id: UserRoleId) -> AppResult<Option<UserRole>>;
    async fn find_by_user(&self, user_id: UserId) -> AppResult<Vec<UserRole>>;
    async fn roles_of_user(&self, user_id: UserId) -> AppResult<Vec<Role>>;
    async fn user_has_role(&self, user_id: UserId, role_id: RoleId) -> AppResult<bool>;
    async fn save(&self, user_role: &UserRole) -> AppResult<()>;
    async fn delete(&self, id: UserRoleId) -> AppResult<()>;
}

#[async_trait]
pub trait RolePermissionRepository: Send + Sync {
    async fn find_by_role(&self, role_id: RoleId) -> AppResult<Vec<String>>;
    async fn find_by_role_names(&self, role_names: &[String]) -> AppResult<Vec<String>>;
    async fn has_permission(
        &self,
        role_names: &[String],
        permission: &str,
    ) -> AppResult<bool>;
    async fn save(&self, rp: &RolePermission) -> AppResult<()>;
    async fn delete(&self, role_id: RoleId, permission: &str) -> AppResult<()>;
    async fn permission_exists(&self, role_id: RoleId, permission: &str) -> AppResult<bool>;
}

#[async_trait]
pub trait RefreshTokenRepository: Send + Sync {
    async fn find_by_hash(&self, token_hash: &str) -> AppResult<Option<RefreshToken>>;
    async fn find_by_id(&self, id: RefreshTokenId) -> AppResult<Option<RefreshToken>>;
    async fn save(&self, token: &RefreshToken) -> AppResult<()>;
}

#[async_trait]
pub trait PasswordResetTokenRepository: Send + Sync {
    async fn find_by_hash(&self, token_hash: &str) -> AppResult<Option<PasswordResetToken>>;
    async fn find_by_id(&self, id: PasswordResetTokenId) -> AppResult<Option<PasswordResetToken>>;
    async fn save(&self, token: &PasswordResetToken) -> AppResult<()>;
}
