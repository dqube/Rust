use std::sync::Arc;

use ddd_shared_kernel::Hasher;

use crate::domain::repositories::{
    PasswordResetTokenRepository, RefreshTokenRepository, RolePermissionRepository,
    RoleRepository, UserRepository, UserRoleRepository,
};
use crate::domain::token_service::TokenService;

/// Dependency container handed to inventory-registered handler factories.
#[derive(Clone)]
pub struct AppDeps {
    pub user_repo: Arc<dyn UserRepository>,
    pub role_repo: Arc<dyn RoleRepository>,
    pub user_role_repo: Arc<dyn UserRoleRepository>,
    pub role_permission_repo: Arc<dyn RolePermissionRepository>,
    pub refresh_token_repo: Arc<dyn RefreshTokenRepository>,
    pub password_reset_repo: Arc<dyn PasswordResetTokenRepository>,
    pub password_hasher: Arc<dyn Hasher>,
    pub token_service: Arc<dyn TokenService>,
    /// Seconds until a password reset token expires. Default 1 hour.
    pub password_reset_ttl_secs: u64,
}
