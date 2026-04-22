//! Postgres-backed implementations of the domain repository ports.
//!
//! Follows the shared-service pattern: one `PgXxxRepository` per port,
//! SeaORM-based, simple mapper fns between DB models and domain entities.

use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ddd_shared_kernel::{AppError, AppResult, Page, PageRequest};
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};

use super::models::*;
use crate::domain::entities::{
    PasswordResetToken, RefreshToken, Role, RolePermission, User, UserRole,
};
use crate::domain::enums::{RoleType, UserType};
use crate::domain::ids::{PasswordResetTokenId, RefreshTokenId, RoleId, UserId, UserRoleId};
use crate::domain::repositories::{
    PasswordResetTokenRepository, RefreshTokenRepository, RolePermissionRepository, RoleRepository,
    UserListFilter, UserRepository, UserRoleRepository,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn to_utc(dt: sea_orm::prelude::DateTimeWithTimeZone) -> DateTime<Utc> {
    dt.with_timezone(&Utc)
}
fn opt_to_utc(dt: Option<sea_orm::prelude::DateTimeWithTimeZone>) -> Option<DateTime<Utc>> {
    dt.map(to_utc)
}
fn from_utc(dt: DateTime<Utc>) -> sea_orm::prelude::DateTimeWithTimeZone {
    dt.fixed_offset()
}
fn opt_from_utc(dt: Option<DateTime<Utc>>) -> Option<sea_orm::prelude::DateTimeWithTimeZone> {
    dt.map(|d| d.fixed_offset())
}
fn db_err(e: sea_orm::DbErr) -> AppError {
    AppError::internal(format!("db: {e}"))
}

// ── Mappers ──────────────────────────────────────────────────────────────────

fn to_user(m: user::Model) -> AppResult<User> {
    let user_type = UserType::from_str(&m.user_type)?;
    Ok(User {
        id: UserId::from_uuid(m.id),
        username: m.username,
        email: m.email,
        email_confirmed: m.email_confirmed,
        phone_number: m.phone_number,
        phone_number_confirmed: m.phone_number_confirmed,
        password_hash: m.password_hash,
        security_stamp: m.security_stamp,
        user_type,
        two_factor_enabled: m.two_factor_enabled,
        two_factor_secret: m.two_factor_secret,
        is_active: m.is_active,
        is_locked: m.is_locked,
        lockout_end: opt_to_utc(m.lockout_end),
        failed_login_attempts: m.failed_login_attempts,
        created_at: to_utc(m.created_at),
        updated_at: opt_to_utc(m.updated_at),
        last_login_at: opt_to_utc(m.last_login_at),
        domain_events: Vec::new(),
    })
}

fn to_role(m: role::Model) -> AppResult<Role> {
    Ok(Role {
        id: RoleId::from_uuid(m.id),
        name: m.name,
        role_type: RoleType::from_str(&m.role_type)?,
        description: m.description,
        is_active: m.is_active,
        created_at: to_utc(m.created_at),
        updated_at: opt_to_utc(m.updated_at),
    })
}

fn to_user_role(m: user_role::Model) -> UserRole {
    UserRole {
        id: UserRoleId::from_uuid(m.id),
        user_id: UserId::from_uuid(m.user_id),
        role_id: RoleId::from_uuid(m.role_id),
        assigned_by: m.assigned_by.map(UserId::from_uuid),
        assigned_at: to_utc(m.assigned_at),
        expires_at: opt_to_utc(m.expires_at),
        domain_events: Vec::new(),
    }
}

fn to_refresh_token(m: refresh_token::Model) -> RefreshToken {
    RefreshToken {
        id: RefreshTokenId::from_uuid(m.id),
        user_id: UserId::from_uuid(m.user_id),
        token_hash: m.token_hash,
        expires_at: to_utc(m.expires_at),
        issued_at: to_utc(m.issued_at),
        revoked_at: opt_to_utc(m.revoked_at),
        replaced_by: m.replaced_by.map(RefreshTokenId::from_uuid),
        ip_address: m.ip_address,
    }
}

fn to_password_reset(m: password_reset_token::Model) -> PasswordResetToken {
    PasswordResetToken {
        id: PasswordResetTokenId::from_uuid(m.id),
        user_id: UserId::from_uuid(m.user_id),
        token_hash: m.token_hash,
        expires_at: to_utc(m.expires_at),
        created_at: to_utc(m.created_at),
        used_at: opt_to_utc(m.used_at),
    }
}

// ── PgUserRepository ─────────────────────────────────────────────────────────

pub struct PgUserRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn find_by_id(&self, id: UserId) -> AppResult<Option<User>> {
        match user::Entity::find_by_id(id.as_uuid())
            .one(&*self.0)
            .await
            .map_err(db_err)?
        {
            Some(m) => Ok(Some(to_user(m)?)),
            None => Ok(None),
        }
    }

    async fn find_by_username(&self, username: &str) -> AppResult<Option<User>> {
        match user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&*self.0)
            .await
            .map_err(db_err)?
        {
            Some(m) => Ok(Some(to_user(m)?)),
            None => Ok(None),
        }
    }

    async fn find_by_email(&self, email: &str) -> AppResult<Option<User>> {
        match user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .one(&*self.0)
            .await
            .map_err(db_err)?
        {
            Some(m) => Ok(Some(to_user(m)?)),
            None => Ok(None),
        }
    }

    async fn find_by_username_or_email(&self, value: &str) -> AppResult<Option<User>> {
        match user::Entity::find()
            .filter(
                Condition::any()
                    .add(user::Column::Username.eq(value))
                    .add(user::Column::Email.eq(value)),
            )
            .one(&*self.0)
            .await
            .map_err(db_err)?
        {
            Some(m) => Ok(Some(to_user(m)?)),
            None => Ok(None),
        }
    }

    async fn email_exists(&self, email: &str) -> AppResult<bool> {
        Ok(user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .count(&*self.0)
            .await
            .map_err(db_err)?
            > 0)
    }

    async fn username_exists(&self, username: &str) -> AppResult<bool> {
        Ok(user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .count(&*self.0)
            .await
            .map_err(db_err)?
            > 0)
    }

    async fn list_paged(
        &self,
        page: PageRequest,
        filter: UserListFilter,
    ) -> AppResult<Page<User>> {
        let mut select = user::Entity::find().order_by_asc(user::Column::CreatedAt);

        if let Some(ref s) = filter.search {
            if !s.is_empty() {
                let like = format!("%{}%", s.to_lowercase());
                select = select.filter(
                    Condition::any()
                        .add(user::Column::Username.contains(like.clone()))
                        .add(user::Column::Email.contains(like)),
                );
            }
        }
        if let Some(active) = filter.is_active {
            select = select.filter(user::Column::IsActive.eq(active));
        }
        if let Some(locked) = filter.is_locked {
            select = select.filter(user::Column::IsLocked.eq(locked));
        }

        let per_page = u64::from(page.per_page().max(1));
        let page_num = u64::from(page.page().max(1)); // 1-based
        let paginator = select.paginate(&*self.0, per_page);
        let total = paginator.num_items().await.map_err(db_err)?;
        let models = paginator
            .fetch_page(page_num - 1)
            .await
            .map_err(db_err)?;

        let mut items = Vec::with_capacity(models.len());
        for m in models {
            items.push(to_user(m)?);
        }
        Ok(Page::new(items, total, page.page(), page.per_page()))
    }

    async fn save(&self, u: &User) -> AppResult<()> {
        let active = user::ActiveModel {
            id: Set(u.id.as_uuid()),
            username: Set(u.username.clone()),
            email: Set(u.email.clone()),
            email_confirmed: Set(u.email_confirmed),
            phone_number: Set(u.phone_number.clone()),
            phone_number_confirmed: Set(u.phone_number_confirmed),
            password_hash: Set(u.password_hash.clone()),
            security_stamp: Set(u.security_stamp.clone()),
            user_type: Set(u.user_type.to_string()),
            two_factor_enabled: Set(u.two_factor_enabled),
            two_factor_secret: Set(u.two_factor_secret.clone()),
            is_active: Set(u.is_active),
            is_locked: Set(u.is_locked),
            lockout_end: Set(opt_from_utc(u.lockout_end)),
            failed_login_attempts: Set(u.failed_login_attempts),
            created_at: Set(from_utc(u.created_at)),
            updated_at: Set(opt_from_utc(u.updated_at)),
            last_login_at: Set(opt_from_utc(u.last_login_at)),
        };
        user::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(user::Column::Id)
                    .update_columns([
                        user::Column::Username,
                        user::Column::Email,
                        user::Column::EmailConfirmed,
                        user::Column::PhoneNumber,
                        user::Column::PhoneNumberConfirmed,
                        user::Column::PasswordHash,
                        user::Column::SecurityStamp,
                        user::Column::UserType,
                        user::Column::TwoFactorEnabled,
                        user::Column::TwoFactorSecret,
                        user::Column::IsActive,
                        user::Column::IsLocked,
                        user::Column::LockoutEnd,
                        user::Column::FailedLoginAttempts,
                        user::Column::UpdatedAt,
                        user::Column::LastLoginAt,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0)
            .await
            .map_err(db_err)?;
        Ok(())
    }
}

// ── PgRoleRepository ────────────────────────────────────────────────────────

pub struct PgRoleRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl RoleRepository for PgRoleRepository {
    async fn find_by_id(&self, id: RoleId) -> AppResult<Option<Role>> {
        match role::Entity::find_by_id(id.as_uuid())
            .one(&*self.0)
            .await
            .map_err(db_err)?
        {
            Some(m) => Ok(Some(to_role(m)?)),
            None => Ok(None),
        }
    }
    async fn find_by_name(&self, name: &str) -> AppResult<Option<Role>> {
        match role::Entity::find()
            .filter(role::Column::Name.eq(name))
            .one(&*self.0)
            .await
            .map_err(db_err)?
        {
            Some(m) => Ok(Some(to_role(m)?)),
            None => Ok(None),
        }
    }
    async fn role_name_exists(&self, name: &str) -> AppResult<bool> {
        Ok(role::Entity::find()
            .filter(role::Column::Name.eq(name))
            .count(&*self.0)
            .await
            .map_err(db_err)?
            > 0)
    }
    async fn list_all(&self) -> AppResult<Vec<Role>> {
        let ms = role::Entity::find()
            .order_by_asc(role::Column::Name)
            .all(&*self.0)
            .await
            .map_err(db_err)?;
        ms.into_iter().map(to_role).collect()
    }
    async fn save(&self, r: &Role) -> AppResult<()> {
        let active = role::ActiveModel {
            id: Set(r.id.as_uuid()),
            name: Set(r.name.clone()),
            role_type: Set(r.role_type.to_string()),
            description: Set(r.description.clone()),
            is_active: Set(r.is_active),
            created_at: Set(from_utc(r.created_at)),
            updated_at: Set(opt_from_utc(r.updated_at)),
        };
        role::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(role::Column::Id)
                    .update_columns([
                        role::Column::Name,
                        role::Column::RoleType,
                        role::Column::Description,
                        role::Column::IsActive,
                        role::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0)
            .await
            .map_err(db_err)?;
        Ok(())
    }
}

// ── PgUserRoleRepository ────────────────────────────────────────────────────

pub struct PgUserRoleRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl UserRoleRepository for PgUserRoleRepository {
    async fn find_by_id(&self, id: UserRoleId) -> AppResult<Option<UserRole>> {
        Ok(user_role::Entity::find_by_id(id.as_uuid())
            .one(&*self.0)
            .await
            .map_err(db_err)?
            .map(to_user_role))
    }

    async fn find_by_user(&self, user_id: UserId) -> AppResult<Vec<UserRole>> {
        Ok(user_role::Entity::find()
            .filter(user_role::Column::UserId.eq(user_id.as_uuid()))
            .order_by_asc(user_role::Column::AssignedAt)
            .all(&*self.0)
            .await
            .map_err(db_err)?
            .into_iter()
            .map(to_user_role)
            .collect())
    }

    async fn roles_of_user(&self, user_id: UserId) -> AppResult<Vec<Role>> {
        // Two-step: fetch user_role rows then load roles by id. Cheap + avoids
        // SeaORM relation wiring we don't need.
        let ur_rows = user_role::Entity::find()
            .filter(user_role::Column::UserId.eq(user_id.as_uuid()))
            .all(&*self.0)
            .await
            .map_err(db_err)?;
        if ur_rows.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<_> = ur_rows.iter().map(|r| r.role_id).collect();
        let roles = role::Entity::find()
            .filter(role::Column::Id.is_in(ids))
            .all(&*self.0)
            .await
            .map_err(db_err)?;
        roles.into_iter().map(to_role).collect()
    }

    async fn user_has_role(&self, user_id: UserId, role_id: RoleId) -> AppResult<bool> {
        Ok(user_role::Entity::find()
            .filter(user_role::Column::UserId.eq(user_id.as_uuid()))
            .filter(user_role::Column::RoleId.eq(role_id.as_uuid()))
            .count(&*self.0)
            .await
            .map_err(db_err)?
            > 0)
    }

    async fn save(&self, ur: &UserRole) -> AppResult<()> {
        let active = user_role::ActiveModel {
            id: Set(ur.id.as_uuid()),
            user_id: Set(ur.user_id.as_uuid()),
            role_id: Set(ur.role_id.as_uuid()),
            assigned_by: Set(ur.assigned_by.map(|a| a.as_uuid())),
            assigned_at: Set(from_utc(ur.assigned_at)),
            expires_at: Set(opt_from_utc(ur.expires_at)),
        };
        user_role::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(user_role::Column::Id)
                    .update_columns([
                        user_role::Column::AssignedBy,
                        user_role::Column::AssignedAt,
                        user_role::Column::ExpiresAt,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn delete(&self, id: UserRoleId) -> AppResult<()> {
        user_role::Entity::delete_by_id(id.as_uuid())
            .exec(&*self.0)
            .await
            .map_err(db_err)?;
        Ok(())
    }
}

// ── PgRolePermissionRepository ──────────────────────────────────────────────

pub struct PgRolePermissionRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl RolePermissionRepository for PgRolePermissionRepository {
    async fn find_by_role(&self, role_id: RoleId) -> AppResult<Vec<String>> {
        Ok(role_permission::Entity::find()
            .filter(role_permission::Column::RoleId.eq(role_id.as_uuid()))
            .order_by_asc(role_permission::Column::Permission)
            .all(&*self.0)
            .await
            .map_err(db_err)?
            .into_iter()
            .map(|m| m.permission)
            .collect())
    }

    async fn find_by_role_names(&self, role_names: &[String]) -> AppResult<Vec<String>> {
        if role_names.is_empty() {
            return Ok(Vec::new());
        }
        let role_ids: Vec<_> = role::Entity::find()
            .filter(role::Column::Name.is_in(role_names.to_vec()))
            .all(&*self.0)
            .await
            .map_err(db_err)?
            .into_iter()
            .map(|r| r.id)
            .collect();
        if role_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut perms: Vec<String> = role_permission::Entity::find()
            .filter(role_permission::Column::RoleId.is_in(role_ids))
            .all(&*self.0)
            .await
            .map_err(db_err)?
            .into_iter()
            .map(|m| m.permission)
            .collect();
        perms.sort();
        perms.dedup();
        Ok(perms)
    }

    async fn has_permission(
        &self,
        role_names: &[String],
        permission: &str,
    ) -> AppResult<bool> {
        if role_names.is_empty() {
            return Ok(false);
        }
        let role_ids: Vec<_> = role::Entity::find()
            .filter(role::Column::Name.is_in(role_names.to_vec()))
            .all(&*self.0)
            .await
            .map_err(db_err)?
            .into_iter()
            .map(|r| r.id)
            .collect();
        if role_ids.is_empty() {
            return Ok(false);
        }
        Ok(role_permission::Entity::find()
            .filter(role_permission::Column::RoleId.is_in(role_ids))
            .filter(role_permission::Column::Permission.eq(permission))
            .count(&*self.0)
            .await
            .map_err(db_err)?
            > 0)
    }

    async fn save(&self, rp: &RolePermission) -> AppResult<()> {
        let active = role_permission::ActiveModel {
            role_id: Set(rp.role_id.as_uuid()),
            permission: Set(rp.permission.clone()),
            created_at: Set(from_utc(rp.created_at)),
        };
        role_permission::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::columns([
                    role_permission::Column::RoleId,
                    role_permission::Column::Permission,
                ])
                .do_nothing()
                .to_owned(),
            )
            .exec_without_returning(&*self.0)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn delete(&self, role_id: RoleId, permission: &str) -> AppResult<()> {
        role_permission::Entity::delete_many()
            .filter(role_permission::Column::RoleId.eq(role_id.as_uuid()))
            .filter(role_permission::Column::Permission.eq(permission))
            .exec(&*self.0)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn permission_exists(
        &self,
        role_id: RoleId,
        permission: &str,
    ) -> AppResult<bool> {
        Ok(role_permission::Entity::find()
            .filter(role_permission::Column::RoleId.eq(role_id.as_uuid()))
            .filter(role_permission::Column::Permission.eq(permission))
            .count(&*self.0)
            .await
            .map_err(db_err)?
            > 0)
    }
}

// ── PgRefreshTokenRepository ────────────────────────────────────────────────

pub struct PgRefreshTokenRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl RefreshTokenRepository for PgRefreshTokenRepository {
    async fn find_by_hash(&self, token_hash: &str) -> AppResult<Option<RefreshToken>> {
        Ok(refresh_token::Entity::find()
            .filter(refresh_token::Column::TokenHash.eq(token_hash))
            .one(&*self.0)
            .await
            .map_err(db_err)?
            .map(to_refresh_token))
    }

    async fn find_by_id(&self, id: RefreshTokenId) -> AppResult<Option<RefreshToken>> {
        Ok(refresh_token::Entity::find_by_id(id.as_uuid())
            .one(&*self.0)
            .await
            .map_err(db_err)?
            .map(to_refresh_token))
    }

    async fn save(&self, t: &RefreshToken) -> AppResult<()> {
        let active = refresh_token::ActiveModel {
            id: Set(t.id.as_uuid()),
            user_id: Set(t.user_id.as_uuid()),
            token_hash: Set(t.token_hash.clone()),
            expires_at: Set(from_utc(t.expires_at)),
            issued_at: Set(from_utc(t.issued_at)),
            revoked_at: Set(opt_from_utc(t.revoked_at)),
            replaced_by: Set(t.replaced_by.map(|r| r.as_uuid())),
            ip_address: Set(t.ip_address.clone()),
        };
        refresh_token::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(refresh_token::Column::Id)
                    .update_columns([
                        refresh_token::Column::TokenHash,
                        refresh_token::Column::ExpiresAt,
                        refresh_token::Column::RevokedAt,
                        refresh_token::Column::ReplacedBy,
                        refresh_token::Column::IpAddress,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0)
            .await
            .map_err(db_err)?;
        Ok(())
    }
}

// ── PgPasswordResetTokenRepository ──────────────────────────────────────────

pub struct PgPasswordResetTokenRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl PasswordResetTokenRepository for PgPasswordResetTokenRepository {
    async fn find_by_hash(&self, token_hash: &str) -> AppResult<Option<PasswordResetToken>> {
        Ok(password_reset_token::Entity::find()
            .filter(password_reset_token::Column::TokenHash.eq(token_hash))
            .one(&*self.0)
            .await
            .map_err(db_err)?
            .map(to_password_reset))
    }

    async fn find_by_id(
        &self,
        id: PasswordResetTokenId,
    ) -> AppResult<Option<PasswordResetToken>> {
        Ok(password_reset_token::Entity::find_by_id(id.as_uuid())
            .one(&*self.0)
            .await
            .map_err(db_err)?
            .map(to_password_reset))
    }

    async fn save(&self, t: &PasswordResetToken) -> AppResult<()> {
        let active = password_reset_token::ActiveModel {
            id: Set(t.id.as_uuid()),
            user_id: Set(t.user_id.as_uuid()),
            token_hash: Set(t.token_hash.clone()),
            expires_at: Set(from_utc(t.expires_at)),
            created_at: Set(from_utc(t.created_at)),
            used_at: Set(opt_from_utc(t.used_at)),
        };
        password_reset_token::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(password_reset_token::Column::Id)
                    .update_columns([
                        password_reset_token::Column::TokenHash,
                        password_reset_token::Column::ExpiresAt,
                        password_reset_token::Column::UsedAt,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0)
            .await
            .map_err(db_err)?;
        Ok(())
    }
}

