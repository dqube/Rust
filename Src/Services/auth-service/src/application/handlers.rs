//! Command + query handlers wired via the mediator.
//!
//! Pattern matches product-service + shared-service: each handler owns the
//! deps it needs and is registered at link time via the `register_*_handler!`
//! macros. Handlers are intentionally thin — business rules live on the
//! aggregates in `crate::domain::entities`.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use ddd_application::{
    register_command_handler, register_query_handler, CommandHandler, QueryHandler,
};
use ddd_shared_kernel::{AppError, AppResult, Hasher, Page, PageRequest};

use super::commands::{
    ActivateUser, AddRolePermission, AssignRole, ChangePassword, ChangePasswordAdmin, CreateRole,
    DeactivateUser, ForgotPassword, Login, LoginResult, Logout,
    RefreshToken as RefreshTokenCmd, Register, RemoveRolePermission, RemoveUserRole,
    ResetPassword,
};
use super::deps::AppDeps;
use super::queries::{
    CheckPermission, GetRolePermissionsById, GetRolePermissionsByNames, GetUser, GetUserByEmail,
    ListRoles, ListUserRoles, ListUsers, UserRoleWithName,
};
use super::rules::{
    email_must_not_exist, password_must_be_valid, permission_must_not_already_exist,
    role_must_exist, role_name_must_not_exist, user_cannot_have_duplicate_role, user_must_exist,
    username_must_not_exist,
};
use crate::domain::entities::{
    PasswordResetToken, RefreshToken as RefreshTokenEntity, Role, RolePermission, User, UserRole,
};
use crate::domain::ids::{RoleId, UserId, UserRoleId};
use crate::domain::repositories::{
    PasswordResetTokenRepository, RefreshTokenRepository, RolePermissionRepository, RoleRepository,
    UserListFilter, UserRepository, UserRoleRepository,
};
use crate::domain::token_service::TokenService;

// ────────────────────────────────────────────────────────────────────────────
// Register
// ────────────────────────────────────────────────────────────────────────────

pub struct RegisterHandler {
    user_repo: Arc<dyn UserRepository>,
    hasher: Arc<dyn Hasher>,
}

#[async_trait]
impl CommandHandler<Register> for RegisterHandler {
    async fn handle(&self, cmd: Register) -> AppResult<UserId> {
        password_must_be_valid(&cmd.password)?;
        email_must_not_exist(&cmd.email, &self.user_repo).await?;
        username_must_not_exist(&cmd.username, &self.user_repo).await?;

        let hash = self.hasher.hash(&cmd.password)?;
        let user = User::register(cmd.username, cmd.email, hash, cmd.user_type, cmd.phone)?;
        let id = user.id;
        self.user_repo.save(&user).await?;
        Ok(id)
    }
}

register_command_handler!(Register, AppDeps, |deps: &AppDeps| {
    RegisterHandler {
        user_repo: deps.user_repo.clone(),
        hasher: deps.password_hasher.clone(),
    }
});

// ────────────────────────────────────────────────────────────────────────────
// Login
// ────────────────────────────────────────────────────────────────────────────

pub struct LoginHandler {
    user_repo: Arc<dyn UserRepository>,
    user_role_repo: Arc<dyn UserRoleRepository>,
    refresh_repo: Arc<dyn RefreshTokenRepository>,
    hasher: Arc<dyn Hasher>,
    token_service: Arc<dyn TokenService>,
}

async fn issue_login(
    deps: &LoginHandler,
    mut user: User,
    ip_address: Option<String>,
) -> AppResult<LoginResult> {
    let roles = deps.user_role_repo.roles_of_user(user.id).await?;
    let role_names: Vec<String> = roles.iter().map(|r| r.name.clone()).collect();
    let tokens = deps
        .token_service
        .issue(
            user.id,
            &user.username,
            &user.email,
            user.user_type,
            &role_names,
        )
        .await?;

    let refresh_entity = RefreshTokenEntity::issue(
        user.id,
        deps.token_service.hash_token(&tokens.refresh_token),
        tokens.refresh_expires_at,
        ip_address,
    );
    deps.refresh_repo.save(&refresh_entity).await?;

    user.record_successful_login();
    deps.user_repo.save(&user).await?;

    Ok(LoginResult {
        tokens,
        user_id: user.id,
        username: user.username,
        email: user.email,
        email_confirmed: user.email_confirmed,
        user_type: user.user_type,
        two_factor_enabled: user.two_factor_enabled,
        is_active: user.is_active,
        is_locked: user.is_locked_out(),
        created_at: user.created_at,
    })
}

#[async_trait]
impl CommandHandler<Login> for LoginHandler {
    async fn handle(&self, cmd: Login) -> AppResult<LoginResult> {
        let Some(mut user) = self
            .user_repo
            .find_by_username_or_email(&cmd.username_or_email)
            .await?
        else {
            return Err(AppError::unauthorized("invalid credentials"));
        };

        if !user.can_login() {
            return Err(AppError::unauthorized(
                "account is locked or deactivated",
            ));
        }
        if !self.hasher.verify(&cmd.password, &user.password_hash)? {
            user.record_failed_login();
            self.user_repo.save(&user).await?;
            return Err(AppError::unauthorized("invalid credentials"));
        }

        issue_login(self, user, cmd.ip_address).await
    }
}

register_command_handler!(Login, AppDeps, |deps: &AppDeps| {
    LoginHandler {
        user_repo: deps.user_repo.clone(),
        user_role_repo: deps.user_role_repo.clone(),
        refresh_repo: deps.refresh_token_repo.clone(),
        hasher: deps.password_hasher.clone(),
        token_service: deps.token_service.clone(),
    }
});

// ────────────────────────────────────────────────────────────────────────────
// RefreshToken
// ────────────────────────────────────────────────────────────────────────────

pub struct RefreshTokenHandler(pub LoginHandler);

#[async_trait]
impl CommandHandler<RefreshTokenCmd> for RefreshTokenHandler {
    async fn handle(&self, cmd: RefreshTokenCmd) -> AppResult<LoginResult> {
        let hash = self.0.token_service.hash_token(&cmd.refresh_token);
        let Some(mut stored) = self.0.refresh_repo.find_by_hash(&hash).await? else {
            return Err(AppError::unauthorized("invalid refresh token"));
        };
        if !stored.is_active() {
            return Err(AppError::unauthorized(
                "refresh token is revoked or expired",
            ));
        }
        let Some(user) = self.0.user_repo.find_by_id(stored.user_id).await? else {
            return Err(AppError::unauthorized("user not found"));
        };
        if !user.can_login() {
            return Err(AppError::unauthorized(
                "account is locked or deactivated",
            ));
        }

        // Rotate: revoke the old refresh, issue a fresh pair.
        stored.revoke(None);
        self.0.refresh_repo.save(&stored).await?;

        issue_login(&self.0, user, cmd.ip_address).await
    }
}

register_command_handler!(RefreshTokenCmd, AppDeps, |deps: &AppDeps| {
    RefreshTokenHandler(LoginHandler {
        user_repo: deps.user_repo.clone(),
        user_role_repo: deps.user_role_repo.clone(),
        refresh_repo: deps.refresh_token_repo.clone(),
        hasher: deps.password_hasher.clone(),
        token_service: deps.token_service.clone(),
    })
});

// ────────────────────────────────────────────────────────────────────────────
// Logout
// ────────────────────────────────────────────────────────────────────────────

pub struct LogoutHandler {
    refresh_repo: Arc<dyn RefreshTokenRepository>,
    token_service: Arc<dyn TokenService>,
}

#[async_trait]
impl CommandHandler<Logout> for LogoutHandler {
    async fn handle(&self, cmd: Logout) -> AppResult<()> {
        let hash = self.token_service.hash_token(&cmd.refresh_token);
        if let Some(mut t) = self.refresh_repo.find_by_hash(&hash).await? {
            if t.is_active() {
                t.revoke(None);
                self.refresh_repo.save(&t).await?;
            }
        }
        Ok(())
    }
}

register_command_handler!(Logout, AppDeps, |deps: &AppDeps| {
    LogoutHandler {
        refresh_repo: deps.refresh_token_repo.clone(),
        token_service: deps.token_service.clone(),
    }
});

// ────────────────────────────────────────────────────────────────────────────
// ChangePassword
// ────────────────────────────────────────────────────────────────────────────

pub struct ChangePasswordHandler {
    user_repo: Arc<dyn UserRepository>,
    hasher: Arc<dyn Hasher>,
}

#[async_trait]
impl CommandHandler<ChangePassword> for ChangePasswordHandler {
    async fn handle(&self, cmd: ChangePassword) -> AppResult<()> {
        password_must_be_valid(&cmd.new_password)?;
        let mut user = self
            .user_repo
            .find_by_id(cmd.user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User", cmd.user_id.to_string()))?;
        if !self.hasher.verify(&cmd.current_password, &user.password_hash)? {
            return Err(AppError::unauthorized("current password is incorrect"));
        }
        let new_hash = self.hasher.hash(&cmd.new_password)?;
        user.update_password(new_hash);
        self.user_repo.save(&user).await
    }
}

register_command_handler!(ChangePassword, AppDeps, |deps: &AppDeps| {
    ChangePasswordHandler {
        user_repo: deps.user_repo.clone(),
        hasher: deps.password_hasher.clone(),
    }
});

// ────────────────────────────────────────────────────────────────────────────
// ChangePasswordAdmin
// ────────────────────────────────────────────────────────────────────────────

pub struct ChangePasswordAdminHandler {
    user_repo: Arc<dyn UserRepository>,
    hasher: Arc<dyn Hasher>,
}

#[async_trait]
impl CommandHandler<ChangePasswordAdmin> for ChangePasswordAdminHandler {
    async fn handle(&self, cmd: ChangePasswordAdmin) -> AppResult<()> {
        password_must_be_valid(&cmd.new_password)?;
        let mut user = self
            .user_repo
            .find_by_id(cmd.user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User", cmd.user_id.to_string()))?;
        let new_hash = self.hasher.hash(&cmd.new_password)?;
        user.update_password(new_hash);
        self.user_repo.save(&user).await
    }
}

register_command_handler!(ChangePasswordAdmin, AppDeps, |deps: &AppDeps| {
    ChangePasswordAdminHandler {
        user_repo: deps.user_repo.clone(),
        hasher: deps.password_hasher.clone(),
    }
});

// ────────────────────────────────────────────────────────────────────────────
// ForgotPassword / ResetPassword
// ────────────────────────────────────────────────────────────────────────────

pub struct ForgotPasswordHandler {
    user_repo: Arc<dyn UserRepository>,
    reset_repo: Arc<dyn PasswordResetTokenRepository>,
    token_service: Arc<dyn TokenService>,
    reset_ttl: Duration,
}

#[async_trait]
impl CommandHandler<ForgotPassword> for ForgotPasswordHandler {
    async fn handle(&self, cmd: ForgotPassword) -> AppResult<()> {
        // Silent success on unknown email — avoids user enumeration.
        let Some(mut user) = self.user_repo.find_by_email(&cmd.email).await? else {
            return Ok(());
        };
        // Random opaque token — the subscriber (email service) delivers it.
        let raw_token = uuid::Uuid::now_v7().to_string().replace('-', "");
        let hash = self.token_service.hash_token(&raw_token);
        let ttl = chrono::Duration::from_std(self.reset_ttl)
            .map_err(|e| AppError::internal(format!("invalid reset_ttl: {e}")))?;
        let reset = PasswordResetToken::issue(user.id, hash, Utc::now() + ttl);
        self.reset_repo.save(&reset).await?;

        // Domain event (eventually flows through the outbox when wired).
        user.emit_password_reset_requested(raw_token);
        // Drain to drop events until the outbox relay is wired in main.rs.
        let _ = user.drain_events();
        Ok(())
    }
}

register_command_handler!(ForgotPassword, AppDeps, |deps: &AppDeps| {
    ForgotPasswordHandler {
        user_repo: deps.user_repo.clone(),
        reset_repo: deps.password_reset_repo.clone(),
        token_service: deps.token_service.clone(),
        reset_ttl: Duration::from_secs(deps.password_reset_ttl_secs),
    }
});

pub struct ResetPasswordHandler {
    user_repo: Arc<dyn UserRepository>,
    reset_repo: Arc<dyn PasswordResetTokenRepository>,
    token_service: Arc<dyn TokenService>,
    hasher: Arc<dyn Hasher>,
}

#[async_trait]
impl CommandHandler<ResetPassword> for ResetPasswordHandler {
    async fn handle(&self, cmd: ResetPassword) -> AppResult<()> {
        password_must_be_valid(&cmd.new_password)?;
        let hash = self.token_service.hash_token(&cmd.token);
        let Some(mut token) = self.reset_repo.find_by_hash(&hash).await? else {
            return Err(AppError::unauthorized("invalid or expired reset token"));
        };
        if !token.is_valid() {
            return Err(AppError::unauthorized("invalid or expired reset token"));
        }

        let mut user = self
            .user_repo
            .find_by_id(token.user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User", token.user_id.to_string()))?;

        let new_hash = self.hasher.hash(&cmd.new_password)?;
        user.update_password(new_hash);
        self.user_repo.save(&user).await?;

        token.mark_used();
        self.reset_repo.save(&token).await?;
        Ok(())
    }
}

register_command_handler!(ResetPassword, AppDeps, |deps: &AppDeps| {
    ResetPasswordHandler {
        user_repo: deps.user_repo.clone(),
        reset_repo: deps.password_reset_repo.clone(),
        token_service: deps.token_service.clone(),
        hasher: deps.password_hasher.clone(),
    }
});

// ────────────────────────────────────────────────────────────────────────────
// Activate / Deactivate
// ────────────────────────────────────────────────────────────────────────────

pub struct ActivateUserHandler {
    user_repo: Arc<dyn UserRepository>,
}

#[async_trait]
impl CommandHandler<ActivateUser> for ActivateUserHandler {
    async fn handle(&self, cmd: ActivateUser) -> AppResult<()> {
        let mut user = self
            .user_repo
            .find_by_id(cmd.user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User", cmd.user_id.to_string()))?;
        user.activate();
        self.user_repo.save(&user).await
    }
}

register_command_handler!(ActivateUser, AppDeps, |deps: &AppDeps| {
    ActivateUserHandler {
        user_repo: deps.user_repo.clone(),
    }
});

pub struct DeactivateUserHandler {
    user_repo: Arc<dyn UserRepository>,
}

#[async_trait]
impl CommandHandler<DeactivateUser> for DeactivateUserHandler {
    async fn handle(&self, cmd: DeactivateUser) -> AppResult<()> {
        let mut user = self
            .user_repo
            .find_by_id(cmd.user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User", cmd.user_id.to_string()))?;
        user.deactivate();
        self.user_repo.save(&user).await
    }
}

register_command_handler!(DeactivateUser, AppDeps, |deps: &AppDeps| {
    DeactivateUserHandler {
        user_repo: deps.user_repo.clone(),
    }
});

// ────────────────────────────────────────────────────────────────────────────
// Roles
// ────────────────────────────────────────────────────────────────────────────

pub struct CreateRoleHandler {
    role_repo: Arc<dyn RoleRepository>,
}

#[async_trait]
impl CommandHandler<CreateRole> for CreateRoleHandler {
    async fn handle(&self, cmd: CreateRole) -> AppResult<RoleId> {
        role_name_must_not_exist(&cmd.name, &self.role_repo).await?;
        let role = Role::create_custom(cmd.name, cmd.description)?;
        let id = role.id;
        self.role_repo.save(&role).await?;
        Ok(id)
    }
}

register_command_handler!(CreateRole, AppDeps, |deps: &AppDeps| {
    CreateRoleHandler {
        role_repo: deps.role_repo.clone(),
    }
});

pub struct AssignRoleHandler {
    user_repo: Arc<dyn UserRepository>,
    role_repo: Arc<dyn RoleRepository>,
    user_role_repo: Arc<dyn UserRoleRepository>,
}

#[async_trait]
impl CommandHandler<AssignRole> for AssignRoleHandler {
    async fn handle(&self, cmd: AssignRole) -> AppResult<UserRoleId> {
        user_must_exist(cmd.user_id, &self.user_repo).await?;
        role_must_exist(cmd.role_id, &self.role_repo).await?;
        user_cannot_have_duplicate_role(cmd.user_id, cmd.role_id, &self.user_role_repo).await?;

        let mut ur = UserRole::assign(cmd.user_id, cmd.role_id, cmd.assigned_by, None);
        let id = ur.id;
        self.user_role_repo.save(&ur).await?;
        let _ = ur.drain_events();
        Ok(id)
    }
}

register_command_handler!(AssignRole, AppDeps, |deps: &AppDeps| {
    AssignRoleHandler {
        user_repo: deps.user_repo.clone(),
        role_repo: deps.role_repo.clone(),
        user_role_repo: deps.user_role_repo.clone(),
    }
});

pub struct RemoveUserRoleHandler {
    user_role_repo: Arc<dyn UserRoleRepository>,
}

#[async_trait]
impl CommandHandler<RemoveUserRole> for RemoveUserRoleHandler {
    async fn handle(&self, cmd: RemoveUserRole) -> AppResult<()> {
        let Some(mut ur) = self.user_role_repo.find_by_id(cmd.user_role_id).await? else {
            return Err(AppError::not_found(
                "UserRole",
                cmd.user_role_id.to_string(),
            ));
        };
        self.user_role_repo.delete(cmd.user_role_id).await?;
        ur.emit_removed();
        let _ = ur.drain_events();
        Ok(())
    }
}

register_command_handler!(RemoveUserRole, AppDeps, |deps: &AppDeps| {
    RemoveUserRoleHandler {
        user_role_repo: deps.user_role_repo.clone(),
    }
});

// ────────────────────────────────────────────────────────────────────────────
// Role permissions
// ────────────────────────────────────────────────────────────────────────────

pub struct AddRolePermissionHandler {
    role_repo: Arc<dyn RoleRepository>,
    perm_repo: Arc<dyn RolePermissionRepository>,
}

#[async_trait]
impl CommandHandler<AddRolePermission> for AddRolePermissionHandler {
    async fn handle(&self, cmd: AddRolePermission) -> AppResult<()> {
        role_must_exist(cmd.role_id, &self.role_repo).await?;
        permission_must_not_already_exist(cmd.role_id, &cmd.permission, &self.perm_repo).await?;

        let rp = RolePermission::new(cmd.role_id, cmd.permission)?;
        self.perm_repo.save(&rp).await
    }
}

register_command_handler!(AddRolePermission, AppDeps, |deps: &AppDeps| {
    AddRolePermissionHandler {
        role_repo: deps.role_repo.clone(),
        perm_repo: deps.role_permission_repo.clone(),
    }
});

pub struct RemoveRolePermissionHandler {
    perm_repo: Arc<dyn RolePermissionRepository>,
}

#[async_trait]
impl CommandHandler<RemoveRolePermission> for RemoveRolePermissionHandler {
    async fn handle(&self, cmd: RemoveRolePermission) -> AppResult<()> {
        self.perm_repo.delete(cmd.role_id, &cmd.permission).await
    }
}

register_command_handler!(RemoveRolePermission, AppDeps, |deps: &AppDeps| {
    RemoveRolePermissionHandler {
        perm_repo: deps.role_permission_repo.clone(),
    }
});

// ────────────────────────────────────────────────────────────────────────────
// Queries
// ────────────────────────────────────────────────────────────────────────────

pub struct GetUserHandler {
    user_repo: Arc<dyn UserRepository>,
}

#[async_trait]
impl QueryHandler<GetUser> for GetUserHandler {
    async fn handle(&self, q: GetUser) -> AppResult<Option<User>> {
        self.user_repo.find_by_id(q.user_id).await
    }
}

register_query_handler!(GetUser, AppDeps, |deps: &AppDeps| {
    GetUserHandler {
        user_repo: deps.user_repo.clone(),
    }
});

pub struct GetUserByEmailHandler {
    user_repo: Arc<dyn UserRepository>,
}

#[async_trait]
impl QueryHandler<GetUserByEmail> for GetUserByEmailHandler {
    async fn handle(&self, q: GetUserByEmail) -> AppResult<Option<User>> {
        self.user_repo.find_by_email(&q.email).await
    }
}

register_query_handler!(GetUserByEmail, AppDeps, |deps: &AppDeps| {
    GetUserByEmailHandler {
        user_repo: deps.user_repo.clone(),
    }
});

pub struct ListUsersHandler {
    user_repo: Arc<dyn UserRepository>,
}

#[async_trait]
impl QueryHandler<ListUsers> for ListUsersHandler {
    async fn handle(&self, q: ListUsers) -> AppResult<Page<User>> {
        let filter = UserListFilter {
            search: q.search,
            is_active: q.is_active,
            is_locked: q.is_locked,
        };
        self.user_repo
            .list_paged(PageRequest::new(q.page, q.per_page), filter)
            .await
    }
}

register_query_handler!(ListUsers, AppDeps, |deps: &AppDeps| {
    ListUsersHandler {
        user_repo: deps.user_repo.clone(),
    }
});

pub struct CheckPermissionHandler {
    user_role_repo: Arc<dyn UserRoleRepository>,
    perm_repo: Arc<dyn RolePermissionRepository>,
}

#[async_trait]
impl QueryHandler<CheckPermission> for CheckPermissionHandler {
    async fn handle(&self, q: CheckPermission) -> AppResult<bool> {
        let roles = self.user_role_repo.roles_of_user(q.user_id).await?;
        if roles.is_empty() {
            return Ok(false);
        }
        let role_names: Vec<String> = roles.into_iter().map(|r| r.name).collect();
        self.perm_repo
            .has_permission(&role_names, &q.permission)
            .await
    }
}

register_query_handler!(CheckPermission, AppDeps, |deps: &AppDeps| {
    CheckPermissionHandler {
        user_role_repo: deps.user_role_repo.clone(),
        perm_repo: deps.role_permission_repo.clone(),
    }
});

pub struct GetRolePermissionsByNamesHandler {
    perm_repo: Arc<dyn RolePermissionRepository>,
}

#[async_trait]
impl QueryHandler<GetRolePermissionsByNames> for GetRolePermissionsByNamesHandler {
    async fn handle(&self, q: GetRolePermissionsByNames) -> AppResult<Vec<String>> {
        self.perm_repo.find_by_role_names(&q.role_names).await
    }
}

register_query_handler!(GetRolePermissionsByNames, AppDeps, |deps: &AppDeps| {
    GetRolePermissionsByNamesHandler {
        perm_repo: deps.role_permission_repo.clone(),
    }
});

pub struct GetRolePermissionsByIdHandler {
    perm_repo: Arc<dyn RolePermissionRepository>,
}

#[async_trait]
impl QueryHandler<GetRolePermissionsById> for GetRolePermissionsByIdHandler {
    async fn handle(&self, q: GetRolePermissionsById) -> AppResult<Vec<String>> {
        self.perm_repo.find_by_role(q.role_id).await
    }
}

register_query_handler!(GetRolePermissionsById, AppDeps, |deps: &AppDeps| {
    GetRolePermissionsByIdHandler {
        perm_repo: deps.role_permission_repo.clone(),
    }
});

pub struct ListRolesHandler {
    role_repo: Arc<dyn RoleRepository>,
}

#[async_trait]
impl QueryHandler<ListRoles> for ListRolesHandler {
    async fn handle(&self, _q: ListRoles) -> AppResult<Vec<Role>> {
        self.role_repo.list_all().await
    }
}

register_query_handler!(ListRoles, AppDeps, |deps: &AppDeps| {
    ListRolesHandler {
        role_repo: deps.role_repo.clone(),
    }
});

pub struct ListUserRolesHandler {
    user_role_repo: Arc<dyn UserRoleRepository>,
    role_repo: Arc<dyn RoleRepository>,
}

#[async_trait]
impl QueryHandler<ListUserRoles> for ListUserRolesHandler {
    async fn handle(&self, q: ListUserRoles) -> AppResult<Vec<UserRoleWithName>> {
        let rows = self.user_role_repo.find_by_user(q.user_id).await?;
        let mut out = Vec::with_capacity(rows.len());
        for ur in rows {
            let (name, rtype) = match self.role_repo.find_by_id(ur.role_id).await? {
                Some(r) => (r.name, r.role_type.to_string()),
                None => (String::new(), String::new()),
            };
            out.push(UserRoleWithName {
                user_role: ur,
                role_name: name,
                role_type: rtype,
            });
        }
        Ok(out)
    }
}

register_query_handler!(ListUserRoles, AppDeps, |deps: &AppDeps| {
    ListUserRolesHandler {
        user_role_repo: deps.user_role_repo.clone(),
        role_repo: deps.role_repo.clone(),
    }
});
