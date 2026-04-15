//! Generic JWT validation.
//!
//! A framework-agnostic validator built on [`jsonwebtoken`]. Supports HMAC and
//! RSA/ECDSA algorithms, verifies signature + standard temporal claims (`exp`,
//! `nbf`), and optionally checks `iss`/`aud`/`sub`. Claims are fully generic so
//! callers can plug in their own struct.
//!
//! Failures are mapped onto [`AppError::Unauthorized`] so they slot straight
//! into the existing REST / gRPC error-mapping.
//!
//! # Example
//!
//! ```ignore
//! use ddd_shared_kernel::jwt::{JwtValidator, StandardClaims};
//!
//! let validator: JwtValidator<StandardClaims> = JwtValidator::hs256(b"secret")
//!     .with_issuer(["https://issuer.example.com"])
//!     .with_audience(["my-api"])
//!     .with_leeway(30);
//!
//! let data = validator.validate(token)?;
//! println!("sub = {}", data.claims.sub);
//! # Ok::<(), ddd_shared_kernel::AppError>(())
//! ```

use std::marker::PhantomData;

use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation, decode};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

// ─── StandardClaims ──────────────────────────────────────────────────────────

/// The registered claims from RFC 7519 plus a `scope` field (common in OAuth 2).
///
/// Use as `JwtValidator<StandardClaims>` when you don't need custom fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StandardClaims {
    /// Subject — whom the token refers to.
    pub sub: String,
    /// Expiration time (UNIX seconds).
    pub exp: i64,
    /// Issued-at time (UNIX seconds).
    #[serde(default)]
    pub iat: Option<i64>,
    /// Not-before time (UNIX seconds).
    #[serde(default)]
    pub nbf: Option<i64>,
    /// Issuer.
    #[serde(default)]
    pub iss: Option<String>,
    /// Audience.
    #[serde(default)]
    pub aud: Option<String>,
    /// Space-separated OAuth 2 scopes.
    #[serde(default)]
    pub scope: Option<String>,
}

// ─── JwtValidator ────────────────────────────────────────────────────────────

/// Generic JWT validator.
///
/// `C` is the claims struct. It must implement [`serde::de::DeserializeOwned`].
pub struct JwtValidator<C> {
    key: DecodingKey,
    validation: Validation,
    _marker: PhantomData<C>,
}

impl<C> JwtValidator<C>
where
    C: for<'de> Deserialize<'de>,
{
    /// HMAC-SHA256 validator (symmetric secret).
    pub fn hs256(secret: &[u8]) -> Self {
        Self::new(DecodingKey::from_secret(secret), Algorithm::HS256)
    }

    /// HMAC-SHA384 validator.
    pub fn hs384(secret: &[u8]) -> Self {
        Self::new(DecodingKey::from_secret(secret), Algorithm::HS384)
    }

    /// HMAC-SHA512 validator.
    pub fn hs512(secret: &[u8]) -> Self {
        Self::new(DecodingKey::from_secret(secret), Algorithm::HS512)
    }

    /// RS256 validator from a PEM-encoded RSA public key.
    pub fn rs256_pem(pem: &[u8]) -> AppResult<Self> {
        let key = DecodingKey::from_rsa_pem(pem)
            .map_err(|e| AppError::internal(format!("invalid RSA public key: {e}")))?;
        Ok(Self::new(key, Algorithm::RS256))
    }

    /// ES256 validator from a PEM-encoded EC public key.
    pub fn es256_pem(pem: &[u8]) -> AppResult<Self> {
        let key = DecodingKey::from_ec_pem(pem)
            .map_err(|e| AppError::internal(format!("invalid EC public key: {e}")))?;
        Ok(Self::new(key, Algorithm::ES256))
    }

    /// Build from an arbitrary key + algorithm (escape hatch for custom setups).
    pub fn new(key: DecodingKey, algorithm: Algorithm) -> Self {
        Self {
            key,
            validation: Validation::new(algorithm),
            _marker: PhantomData,
        }
    }

    /// Require one of the given issuers in the `iss` claim.
    #[must_use]
    pub fn with_issuer<I, S>(mut self, issuers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.validation
            .set_issuer(&issuers.into_iter().map(Into::into).collect::<Vec<_>>());
        self
    }

    /// Require one of the given audiences in the `aud` claim.
    #[must_use]
    pub fn with_audience<I, S>(mut self, audiences: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.validation
            .set_audience(&audiences.into_iter().map(Into::into).collect::<Vec<_>>());
        self
    }

    /// Require one of the given algorithms (replaces the default).
    ///
    /// Useful when rotating keys or accepting both RS256 and ES256 during a
    /// migration.
    #[must_use]
    pub fn with_algorithms(mut self, algorithms: Vec<Algorithm>) -> Self {
        self.validation.algorithms = algorithms;
        self
    }

    /// Clock skew tolerance in seconds for `exp` / `nbf` (default: 0).
    #[must_use]
    pub fn with_leeway(mut self, seconds: u64) -> Self {
        self.validation.leeway = seconds;
        self
    }

    /// Require a specific `sub` claim value.
    #[must_use]
    pub fn with_required_sub(mut self, sub: impl Into<String>) -> Self {
        self.validation.sub = Some(sub.into());
        self
    }

    /// Skip signature validation — **do not use in production**.
    ///
    /// Intended for tests and local development only.
    #[must_use]
    pub fn insecure_disable_signature_check(mut self) -> Self {
        self.validation.insecure_disable_signature_validation();
        self
    }

    /// Validate `token` (without the `Bearer ` prefix).
    ///
    /// On success returns the decoded header + claims. On failure returns an
    /// [`AppError::Unauthorized`] whose message identifies the cause.
    pub fn validate(&self, token: &str) -> AppResult<TokenData<C>> {
        decode::<C>(token, &self.key, &self.validation).map_err(map_jwt_error)
    }
}

// ─── Error mapping ───────────────────────────────────────────────────────────

fn map_jwt_error(err: jsonwebtoken::errors::Error) -> AppError {
    let message = match err.kind() {
        ErrorKind::InvalidToken => "invalid token",
        ErrorKind::InvalidSignature => "invalid token signature",
        ErrorKind::ExpiredSignature => "token expired",
        ErrorKind::ImmatureSignature => "token not yet valid",
        ErrorKind::InvalidIssuer => "invalid token issuer",
        ErrorKind::InvalidAudience => "invalid token audience",
        ErrorKind::InvalidSubject => "invalid token subject",
        ErrorKind::InvalidAlgorithm | ErrorKind::InvalidAlgorithmName => "invalid token algorithm",
        ErrorKind::MissingRequiredClaim(_) => "token missing required claim",
        ErrorKind::Base64(_) | ErrorKind::Utf8(_) | ErrorKind::Json(_) => "malformed token",
        _ => "token validation failed",
    };
    AppError::unauthorized(message)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{EncodingKey, Header, encode};

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    fn mint(secret: &[u8], claims: &StandardClaims) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(secret),
        )
        .unwrap()
    }

    #[test]
    fn accepts_valid_hs256_token() {
        let secret = b"supersecret";
        let claims = StandardClaims {
            sub: "user-1".into(),
            exp: now() + 60,
            iat: Some(now()),
            nbf: None,
            iss: Some("issuer-a".into()),
            aud: Some("my-api".into()),
            scope: Some("read write".into()),
        };
        let token = mint(secret, &claims);

        let validator: JwtValidator<StandardClaims> = JwtValidator::hs256(secret)
            .with_issuer(["issuer-a"])
            .with_audience(["my-api"]);
        let data = validator.validate(&token).expect("valid");
        assert_eq!(data.claims.sub, "user-1");
    }

    #[test]
    fn rejects_expired_token() {
        let secret = b"s";
        let claims = StandardClaims {
            sub: "u".into(),
            exp: now() - 3600,
            iat: None,
            nbf: None,
            iss: None,
            aud: None,
            scope: None,
        };
        let token = mint(secret, &claims);
        let validator: JwtValidator<StandardClaims> =
            JwtValidator::hs256(secret).with_leeway(0);
        let err = validator.validate(&token).unwrap_err();
        assert!(matches!(err, AppError::Unauthorized { ref message } if message == "token expired"));
    }

    #[test]
    fn rejects_wrong_signature() {
        let claims = StandardClaims {
            sub: "u".into(),
            exp: now() + 60,
            iat: None,
            nbf: None,
            iss: None,
            aud: None,
            scope: None,
        };
        let token = mint(b"right", &claims);
        let validator: JwtValidator<StandardClaims> = JwtValidator::hs256(b"wrong");
        let err = validator.validate(&token).unwrap_err();
        assert!(matches!(err, AppError::Unauthorized { .. }));
    }

    #[test]
    fn rejects_wrong_issuer() {
        let secret = b"s";
        let claims = StandardClaims {
            sub: "u".into(),
            exp: now() + 60,
            iat: None,
            nbf: None,
            iss: Some("bad".into()),
            aud: None,
            scope: None,
        };
        let token = mint(secret, &claims);
        let validator: JwtValidator<StandardClaims> =
            JwtValidator::hs256(secret).with_issuer(["good"]);
        let err = validator.validate(&token).unwrap_err();
        assert!(matches!(err, AppError::Unauthorized { ref message } if message == "invalid token issuer"));
    }

    #[test]
    fn malformed_token_is_unauthorized() {
        let validator: JwtValidator<StandardClaims> = JwtValidator::hs256(b"s");
        let err = validator.validate("not.a.jwt").unwrap_err();
        assert!(matches!(err, AppError::Unauthorized { .. }));
    }

    #[test]
    fn leeway_allows_just_expired_token() {
        let secret = b"s";
        let claims = StandardClaims {
            sub: "u".into(),
            exp: now() - 5,
            iat: None,
            nbf: None,
            iss: None,
            aud: None,
            scope: None,
        };
        let token = mint(secret, &claims);
        let validator: JwtValidator<StandardClaims> =
            JwtValidator::hs256(secret).with_leeway(60);
        assert!(validator.validate(&token).is_ok());
    }

    #[test]
    fn supports_custom_claims() {
        #[derive(Serialize, Deserialize)]
        struct MyClaims {
            sub: String,
            exp: i64,
            tenant: String,
        }
        let secret = b"s";
        let claims = MyClaims {
            sub: "u".into(),
            exp: now() + 60,
            tenant: "acme".into(),
        };
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret),
        )
        .unwrap();
        let validator: JwtValidator<MyClaims> = JwtValidator::hs256(secret);
        let data = validator.validate(&token).expect("valid");
        assert_eq!(data.claims.tenant, "acme");
    }
}
