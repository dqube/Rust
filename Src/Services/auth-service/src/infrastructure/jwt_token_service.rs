//! JWT-based [`TokenService`] adapter.
//!
//! - **Access tokens** are short-lived HS256 JWTs signed with the same
//!   secret the BFF uses to validate them (via
//!   [`ddd_shared_kernel::jwt::JwtValidator`]), so admin-bff can accept
//!   tokens minted here without any cross-service key exchange.
//! - **Refresh tokens** are opaque random URL-safe base64 strings. We
//!   persist only their SHA-256 hash so a DB leak cannot replay them.
//! - Token hashing uses plain SHA-256 (fast, non-secret-input — the
//!   random token itself is the secret). Password hashes use PBKDF2
//!   through [`ddd_infrastructure::Pbkdf2Hasher`] instead.

use std::time::Duration;

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use ddd_shared_kernel::{AppError, AppResult};
use jsonwebtoken::{encode, EncodingKey, Header};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::enums::UserType;
use crate::domain::ids::UserId;
use crate::domain::token_service::{IssuedTokens, TokenService};

/// Number of random bytes used to generate a refresh/reset token.
const REFRESH_TOKEN_BYTES: usize = 32;

/// JWT claims encoded into every access token.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccessClaims {
    /// Subject — user id (UUID as string).
    sub: String,
    /// Username — convenience for downstream services.
    username: String,
    /// Email.
    email: String,
    /// User type (Customer / Employee / Supplier / Admin).
    user_type: String,
    /// Roles assigned to the user at issue time.
    roles: Vec<String>,
    /// Issuer.
    iss: String,
    /// Audience.
    aud: String,
    /// Expiry (unix seconds).
    exp: i64,
    /// Issued-at (unix seconds).
    iat: i64,
    /// Not-before (unix seconds).
    nbf: i64,
}

/// Configuration for [`JwtTokenService`].
#[derive(Debug, Clone)]
pub struct JwtTokenServiceConfig {
    pub secret: String,
    pub issuer: String,
    pub audience: String,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

pub struct JwtTokenService {
    encoding_key: EncodingKey,
    issuer: String,
    audience: String,
    access_ttl: chrono::Duration,
    refresh_ttl: chrono::Duration,
}

impl JwtTokenService {
    pub fn new(cfg: JwtTokenServiceConfig) -> AppResult<Self> {
        if cfg.secret.len() < 32 {
            return Err(AppError::internal(
                "JWT secret must be at least 32 bytes for HS256",
            ));
        }
        Ok(Self {
            encoding_key: EncodingKey::from_secret(cfg.secret.as_bytes()),
            issuer: cfg.issuer,
            audience: cfg.audience,
            access_ttl: chrono::Duration::from_std(cfg.access_ttl)
                .map_err(|e| AppError::internal(format!("invalid access_ttl: {e}")))?,
            refresh_ttl: chrono::Duration::from_std(cfg.refresh_ttl)
                .map_err(|e| AppError::internal(format!("invalid refresh_ttl: {e}")))?,
        })
    }

    fn random_refresh_token() -> String {
        let mut bytes = [0u8; REFRESH_TOKEN_BYTES];
        rand::thread_rng().fill_bytes(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }
}

#[async_trait]
impl TokenService for JwtTokenService {
    async fn issue(
        &self,
        user_id: UserId,
        username: &str,
        email: &str,
        user_type: UserType,
        roles: &[String],
    ) -> AppResult<IssuedTokens> {
        let now = Utc::now();
        let access_expires_at = now + self.access_ttl;
        let refresh_expires_at = now + self.refresh_ttl;

        let claims = AccessClaims {
            sub: user_id.to_string(),
            username: username.to_owned(),
            email: email.to_owned(),
            user_type: user_type.to_string(),
            roles: roles.to_vec(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            exp: access_expires_at.timestamp(),
        };

        let access_token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AppError::internal(format!("jwt encode: {e}")))?;

        let refresh_token = Self::random_refresh_token();

        Ok(IssuedTokens {
            access_token,
            refresh_token,
            access_expires_at,
            refresh_expires_at,
        })
    }

    fn hash_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token);
        URL_SAFE_NO_PAD.encode(hasher.finalize())
    }
}
