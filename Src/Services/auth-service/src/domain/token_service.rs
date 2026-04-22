//! Token issuance / verification port.
//!
//! Issuance is domain-relevant (roles baked into the access token), so the
//! abstraction lives here. The adapter in `infrastructure::jwt_token_service`
//! uses the `jsonwebtoken` crate for signing and delegates verification to
//! [`ddd_shared_kernel::jwt::JwtValidator`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ddd_shared_kernel::AppResult;

use super::enums::UserType;
use super::ids::UserId;

/// Outcome of a successful token issue.
#[derive(Debug, Clone)]
pub struct IssuedTokens {
    /// Short-lived signed JWT bearer access token.
    pub access_token: String,
    /// Opaque refresh token — callers must hash before persisting.
    pub refresh_token: String,
    /// Access-token expiry.
    pub access_expires_at: DateTime<Utc>,
    /// Refresh-token expiry.
    pub refresh_expires_at: DateTime<Utc>,
}

#[async_trait]
pub trait TokenService: Send + Sync {
    /// Mint an access token + refresh token for the given user.
    async fn issue(
        &self,
        user_id: UserId,
        username: &str,
        email: &str,
        user_type: UserType,
        roles: &[String],
    ) -> AppResult<IssuedTokens>;

    /// Hash a refresh/reset token for persistence. The same adapter must be
    /// used on both the write and the read side so stored hashes compare.
    fn hash_token(&self, token: &str) -> String;
}
