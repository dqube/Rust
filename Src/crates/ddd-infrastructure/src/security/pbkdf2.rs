//! [`Hasher`] adapter using PBKDF2-SHA256 with a per-call random salt.
//!
//! Output is the standard PHC-encoded string (`$pbkdf2-sha256$…`) produced
//! by the [`password_hash`] crate, so verification needs only the
//! candidate plaintext and the stored hash.

use ddd_shared_kernel::{AppError, AppResult, Hasher};
use pbkdf2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use pbkdf2::Pbkdf2;

/// PBKDF2-SHA256 [`Hasher`] adapter.
///
/// The OWASP-recommended default is ~600 000 iterations of PBKDF2-SHA256;
/// the underlying [`pbkdf2::Params::default`] currently mirrors that.
#[derive(Debug, Default, Clone, Copy)]
pub struct Pbkdf2Hasher;

impl Pbkdf2Hasher {
    /// Construct the default PBKDF2 hasher.
    pub fn new() -> Self {
        Self
    }
}

impl Hasher for Pbkdf2Hasher {
    fn hash(&self, plain: &str) -> AppResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        Pbkdf2
            .hash_password(plain.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| AppError::internal(format!("pbkdf2 hash: {e}")))
    }

    fn verify(&self, plain: &str, hash: &str) -> AppResult<bool> {
        let parsed = PasswordHash::new(hash)
            .map_err(|e| AppError::internal(format!("pbkdf2 parse: {e}")))?;
        Ok(Pbkdf2.verify_password(plain.as_bytes(), &parsed).is_ok())
    }
}
