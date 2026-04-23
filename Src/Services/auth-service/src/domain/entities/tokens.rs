use chrono::{DateTime, Utc};

use crate::domain::ids::{PasswordResetTokenId, RefreshTokenId, UserId};

// ── RefreshToken ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RefreshToken {
    pub id:          RefreshTokenId,
    pub user_id:     UserId,
    pub token_hash:  String,
    pub expires_at:  DateTime<Utc>,
    pub issued_at:   DateTime<Utc>,
    pub revoked_at:  Option<DateTime<Utc>>,
    pub replaced_by: Option<RefreshTokenId>,
    pub ip_address:  Option<String>,
}

impl RefreshToken {
    pub fn issue(
        user_id:    UserId,
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

// ── PasswordResetToken ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PasswordResetToken {
    pub id:         PasswordResetTokenId,
    pub user_id:    UserId,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub used_at:    Option<DateTime<Utc>>,
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
