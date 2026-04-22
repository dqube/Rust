//! Auth domain entities.
//!
//! Kept as plain structs with pure methods (no mediator/repository coupling)
//! — the application layer orchestrates persistence and event publication.
//! Password material is stored as a PHC-encoded hash string produced by the
//! [`Hasher`](ddd_shared_kernel::Hasher) port, so there is no separate salt
//! field (the PHC string embeds it).

use chrono::{DateTime, Utc};
use ddd_shared_kernel::{AppError, AppResult, DomainEvent};

use super::enums::{RoleType, UserType};
use super::events::{
    PasswordResetRequested, UserActivated, UserDeactivated, UserPasswordChanged, UserRegistered,
    UserRoleAssigned, UserRoleRemoved,
};
use super::ids::{PasswordResetTokenId, RefreshTokenId, RoleId, UserId, UserRoleId};

/// Lockout threshold — five consecutive failures locks the account.
pub const LOCKOUT_THRESHOLD: i32 = 5;
/// Lockout duration.
pub const LOCKOUT_MINUTES: i64 = 30;

// ── User ────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub email: String,
    pub email_confirmed: bool,
    pub phone_number: Option<String>,
    pub phone_number_confirmed: bool,
    pub password_hash: String, // PHC-encoded
    pub security_stamp: String,
    pub user_type: UserType,
    pub two_factor_enabled: bool,
    pub two_factor_secret: Option<String>,
    pub is_active: bool,
    pub is_locked: bool,
    pub lockout_end: Option<DateTime<Utc>>,
    pub failed_login_attempts: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub last_login_at: Option<DateTime<Utc>>,

    #[doc(hidden)]
    pub domain_events: Vec<Box<dyn DomainEvent>>,
}

impl Clone for User {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            username: self.username.clone(),
            email: self.email.clone(),
            email_confirmed: self.email_confirmed,
            phone_number: self.phone_number.clone(),
            phone_number_confirmed: self.phone_number_confirmed,
            password_hash: self.password_hash.clone(),
            security_stamp: self.security_stamp.clone(),
            user_type: self.user_type,
            two_factor_enabled: self.two_factor_enabled,
            two_factor_secret: self.two_factor_secret.clone(),
            is_active: self.is_active,
            is_locked: self.is_locked,
            lockout_end: self.lockout_end,
            failed_login_attempts: self.failed_login_attempts,
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_login_at: self.last_login_at,
            // Domain events are runtime side effects and are intentionally not cloned.
            domain_events: Vec::new(),
        }
    }
}

impl User {
    /// Register a new user. Caller supplies a PHC-encoded password hash.
    #[allow(clippy::too_many_arguments)]
    pub fn register(
        username: String,
        email: String,
        password_hash: String,
        user_type: UserType,
        phone: Option<String>,
    ) -> AppResult<Self> {
        if username.trim().is_empty() {
            return Err(AppError::validation("username", "must not be empty"));
        }
        if email.trim().is_empty() {
            return Err(AppError::validation("email", "must not be empty"));
        }
        if !email.contains('@') {
            return Err(AppError::validation("email", "invalid email format"));
        }
        let id = UserId::new();
        let now = Utc::now();
        let mut u = Self {
            id,
            username: username.clone(),
            email: email.clone(),
            email_confirmed: false,
            phone_number: phone,
            phone_number_confirmed: false,
            password_hash,
            security_stamp: uuid::Uuid::now_v7().to_string(),
            user_type,
            two_factor_enabled: false,
            two_factor_secret: None,
            is_active: true,
            is_locked: false,
            lockout_end: None,
            failed_login_attempts: 0,
            created_at: now,
            updated_at: None,
            last_login_at: None,
            domain_events: Vec::new(),
        };
        u.domain_events.push(Box::new(UserRegistered {
            user_id: id,
            username,
            email,
            user_type,
            occurred_at: now,
        }));
        Ok(u)
    }

    pub fn update_password(&mut self, new_hash: String) {
        self.password_hash = new_hash;
        self.failed_login_attempts = 0;
        self.is_locked = false;
        self.lockout_end = None;
        self.security_stamp = uuid::Uuid::now_v7().to_string();
        self.updated_at = Some(Utc::now());
        self.domain_events.push(Box::new(UserPasswordChanged {
            user_id: self.id,
            occurred_at: Utc::now(),
        }));
    }

    pub fn confirm_email(&mut self) {
        self.email_confirmed = true;
        self.updated_at = Some(Utc::now());
    }

    pub fn enable_two_factor(&mut self, secret: String) {
        self.two_factor_enabled = true;
        self.two_factor_secret = Some(secret);
        self.security_stamp = uuid::Uuid::now_v7().to_string();
        self.updated_at = Some(Utc::now());
    }

    pub fn disable_two_factor(&mut self) {
        self.two_factor_enabled = false;
        self.two_factor_secret = None;
        self.security_stamp = uuid::Uuid::now_v7().to_string();
        self.updated_at = Some(Utc::now());
    }

    /// Records a failed login attempt. Locks the account after
    /// [`LOCKOUT_THRESHOLD`] failures for [`LOCKOUT_MINUTES`] minutes.
    pub fn record_failed_login(&mut self) {
        self.failed_login_attempts += 1;
        if self.failed_login_attempts >= LOCKOUT_THRESHOLD {
            self.is_locked = true;
            self.lockout_end = Some(Utc::now() + chrono::Duration::minutes(LOCKOUT_MINUTES));
        }
        self.updated_at = Some(Utc::now());
    }

    pub fn record_successful_login(&mut self) {
        self.failed_login_attempts = 0;
        self.is_locked = false;
        self.lockout_end = None;
        self.last_login_at = Some(Utc::now());
        self.updated_at = Some(Utc::now());
    }

    pub fn activate(&mut self) {
        if self.is_active {
            return;
        }
        self.is_active = true;
        self.failed_login_attempts = 0;
        self.is_locked = false;
        self.lockout_end = None;
        self.updated_at = Some(Utc::now());
        self.domain_events.push(Box::new(UserActivated {
            user_id: self.id,
            occurred_at: Utc::now(),
        }));
    }

    pub fn deactivate(&mut self) {
        if !self.is_active {
            return;
        }
        self.is_active = false;
        self.updated_at = Some(Utc::now());
        self.domain_events.push(Box::new(UserDeactivated {
            user_id: self.id,
            occurred_at: Utc::now(),
        }));
    }

    /// GDPR anonymisation — wipes all PII. Callers must still persist and
    /// typically follow up with account deletion or deactivation.
    pub fn anonymize_for_gdpr(&mut self) {
        let suffix = self.id.as_uuid().simple().to_string();
        self.username = format!("deleted-{suffix}");
        self.email = format!("deleted-{suffix}@gdpr.local");
        self.phone_number = None;
        self.phone_number_confirmed = false;
        self.two_factor_secret = None;
        self.two_factor_enabled = false;
        self.password_hash = String::new();
        self.security_stamp = uuid::Uuid::now_v7().to_string();
        self.updated_at = Some(Utc::now());
    }

    pub fn is_locked_out(&self) -> bool {
        self.is_locked
            || self
                .lockout_end
                .is_some_and(|end| end > Utc::now())
    }

    pub fn can_login(&self) -> bool {
        self.is_active && !self.is_locked_out()
    }

    pub fn emit_password_reset_requested(&mut self, reset_token: String) {
        self.domain_events.push(Box::new(PasswordResetRequested {
            user_id: self.id,
            email: self.email.clone(),
            reset_token,
            occurred_at: Utc::now(),
        }));
    }

    pub fn drain_events(&mut self) -> Vec<Box<dyn DomainEvent>> {
        std::mem::take(&mut self.domain_events)
    }
}

// ── Role ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Role {
    pub id: RoleId,
    pub name: String,
    pub role_type: RoleType,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl Role {
    pub fn create_custom(name: String, description: Option<String>) -> AppResult<Self> {
        let name = name.trim().to_owned();
        if name.is_empty() {
            return Err(AppError::validation("name", "must not be empty"));
        }
        if name.len() > 100 {
            return Err(AppError::validation(
                "name",
                "must be at most 100 characters",
            ));
        }
        Ok(Self {
            id: RoleId::new(),
            name,
            role_type: RoleType::Custom,
            description,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
        })
    }

    pub fn builtin(id: RoleId, name: &str, description: &str) -> Self {
        Self {
            id,
            name: name.to_owned(),
            role_type: RoleType::BuiltIn,
            description: Some(description.to_owned()),
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
        }
    }
}

// ── UserRole ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct UserRole {
    pub id: UserRoleId,
    pub user_id: UserId,
    pub role_id: RoleId,
    pub assigned_by: Option<UserId>,
    pub assigned_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,

    #[doc(hidden)]
    pub domain_events: Vec<Box<dyn DomainEvent>>,
}

impl Clone for UserRole {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            user_id: self.user_id,
            role_id: self.role_id,
            assigned_by: self.assigned_by,
            assigned_at: self.assigned_at,
            expires_at: self.expires_at,
            // Domain events are runtime side effects and are intentionally not cloned.
            domain_events: Vec::new(),
        }
    }
}

impl UserRole {
    pub fn assign(
        user_id: UserId,
        role_id: RoleId,
        assigned_by: Option<UserId>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Self {
        let id = UserRoleId::new();
        let now = Utc::now();
        let mut ur = Self {
            id,
            user_id,
            role_id,
            assigned_by,
            assigned_at: now,
            expires_at,
            domain_events: Vec::new(),
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

// ── RolePermission ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RolePermission {
    pub role_id: RoleId,
    pub permission: String,
    pub created_at: DateTime<Utc>,
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

// ── RefreshToken ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RefreshToken {
    pub id: RefreshTokenId,
    pub user_id: UserId,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub issued_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub replaced_by: Option<RefreshTokenId>,
    pub ip_address: Option<String>,
}

impl RefreshToken {
    pub fn issue(
        user_id: UserId,
        token_hash: String,
        expires_at: DateTime<Utc>,
        ip_address: Option<String>,
    ) -> Self {
        Self {
            id: RefreshTokenId::new(),
            user_id,
            token_hash,
            expires_at,
            issued_at: Utc::now(),
            revoked_at: None,
            replaced_by: None,
            ip_address,
        }
    }

    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none() && self.expires_at > Utc::now()
    }

    pub fn revoke(&mut self, replaced_by: Option<RefreshTokenId>) {
        self.revoked_at = Some(Utc::now());
        self.replaced_by = replaced_by;
    }
}

// ── PasswordResetToken ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PasswordResetToken {
    pub id: PasswordResetTokenId,
    pub user_id: UserId,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

impl PasswordResetToken {
    pub fn issue(user_id: UserId, token_hash: String, expires_at: DateTime<Utc>) -> Self {
        Self {
            id: PasswordResetTokenId::new(),
            user_id,
            token_hash,
            expires_at,
            created_at: Utc::now(),
            used_at: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.used_at.is_none() && self.expires_at > Utc::now()
    }

    pub fn mark_used(&mut self) {
        self.used_at = Some(Utc::now());
    }
}
