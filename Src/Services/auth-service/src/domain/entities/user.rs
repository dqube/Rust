use chrono::{DateTime, Utc};

use ddd_domain::{define_aggregate, impl_aggregate, impl_aggregate_events};
use ddd_shared_kernel::{AppError, AppResult, DomainEvent};

use crate::domain::enums::UserType;
use crate::domain::events::{
    PasswordResetRequested, UserActivated, UserDeactivated, UserPasswordChanged, UserRegistered,
};
use crate::domain::ids::UserId;

pub const LOCKOUT_THRESHOLD: i32 = 5;
pub const LOCKOUT_MINUTES: i64 = 30;

// ── User ──────────────────────────────────────────────────────────────────────

define_aggregate!(User, UserId, {
    pub username:                String,
    pub email:                   String,
    pub email_confirmed:         bool,
    pub phone_number:            Option<String>,
    pub phone_number_confirmed:  bool,
    pub password_hash:           String,
    pub security_stamp:          String,
    pub user_type:               UserType,
    pub two_factor_enabled:      bool,
    pub two_factor_secret:       Option<String>,
    pub is_active:               bool,
    pub is_locked:               bool,
    pub lockout_end:             Option<DateTime<Utc>>,
    pub failed_login_attempts:   i32,
    pub last_login_at:           Option<DateTime<Utc>>,
});

impl_aggregate!(User, UserId);
impl_aggregate_events!(User);

impl Clone for User {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            version: self.version,
            created_at: self.created_at,
            updated_at: self.updated_at,
            domain_events: Vec::new(),
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
            last_login_at: self.last_login_at,
        }
    }
}

impl User {
    #[allow(clippy::too_many_arguments)]
    pub fn register(
        username:      String,
        email:         String,
        password_hash: String,
        user_type:     UserType,
        phone:         Option<String>,
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
            version: 0,
            domain_events: Vec::new(),
            created_at: now,
            updated_at: now,
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
            last_login_at: None,
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
        self.updated_at = Utc::now();
        self.domain_events.push(Box::new(UserPasswordChanged {
            user_id: self.id,
            occurred_at: Utc::now(),
        }));
    }

    pub fn confirm_email(&mut self) {
        self.email_confirmed = true;
        self.updated_at = Utc::now();
    }

    pub fn enable_two_factor(&mut self, secret: String) {
        self.two_factor_enabled = true;
        self.two_factor_secret = Some(secret);
        self.security_stamp = uuid::Uuid::now_v7().to_string();
        self.updated_at = Utc::now();
    }

    pub fn disable_two_factor(&mut self) {
        self.two_factor_enabled = false;
        self.two_factor_secret = None;
        self.security_stamp = uuid::Uuid::now_v7().to_string();
        self.updated_at = Utc::now();
    }

    pub fn record_failed_login(&mut self) {
        self.failed_login_attempts += 1;
        if self.failed_login_attempts >= LOCKOUT_THRESHOLD {
            self.is_locked = true;
            self.lockout_end = Some(Utc::now() + chrono::Duration::minutes(LOCKOUT_MINUTES));
        }
        self.updated_at = Utc::now();
    }

    pub fn record_successful_login(&mut self) {
        self.failed_login_attempts = 0;
        self.is_locked = false;
        self.lockout_end = None;
        self.last_login_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn activate(&mut self) {
        if self.is_active {
            return;
        }
        self.is_active = true;
        self.failed_login_attempts = 0;
        self.is_locked = false;
        self.lockout_end = None;
        self.updated_at = Utc::now();
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
        self.updated_at = Utc::now();
        self.domain_events.push(Box::new(UserDeactivated {
            user_id: self.id,
            occurred_at: Utc::now(),
        }));
    }

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
        self.updated_at = Utc::now();
    }

    pub fn is_locked_out(&self) -> bool {
        self.is_locked || self.lockout_end.is_some_and(|end| end > Utc::now())
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
