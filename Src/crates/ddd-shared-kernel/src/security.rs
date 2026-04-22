//! Security ports: password hashing and symmetric encryption.
//!
//! Two small driven ports so the domain / application can ask "hash this
//! password" or "encrypt this blob" without depending on a particular
//! algorithm or crate.  Concrete adapters live in `ddd-infrastructure`
//! behind the `crypto` feature (PBKDF2-SHA256, AES-256-GCM today).
//!
//! # Design choices
//!
//! - [`Hasher`] is **synchronous** — password hashing is CPU-bound and the
//!   adapter is expected to be wrapped in `tokio::task::spawn_blocking` by
//!   callers when needed.
//! - [`Cipher`] is **async** — encryption is also CPU-bound today, but
//!   modelling it `async` leaves room for KMS-backed implementations
//!   (AWS KMS, Azure Key Vault) without breaking the trait.
//! - Errors flow through [`AppError`](crate::AppError) — adapters map crate
//!   errors to [`AppError::internal`](crate::AppError::internal) for
//!   transport failures and [`AppError::unauthorized`](crate::AppError::unauthorized)
//!   for verification failures the *caller* must distinguish from a clean
//!   `Ok(false)`.

use async_trait::async_trait;

use crate::AppResult;

/// Password / secret hashing port.
///
/// Implementations produce self-describing PHC-style strings that include
/// the algorithm, parameters, and salt — verification therefore needs only
/// the candidate plaintext and the stored hash.
pub trait Hasher: Send + Sync {
    /// Hash `plain` and return the encoded representation.
    ///
    /// # Errors
    /// Returns [`AppError::internal`](crate::AppError::internal) when the
    /// underlying primitive fails (e.g. RNG failure).
    fn hash(&self, plain: &str) -> AppResult<String>;

    /// Verify `plain` against a previously computed `hash`.  Returns
    /// `Ok(true)` for a match, `Ok(false)` for a clean mismatch, and
    /// `Err(_)` only when the hash string itself is malformed.
    fn verify(&self, plain: &str, hash: &str) -> AppResult<bool>;
}

/// Symmetric encryption port.
///
/// The [`encrypt`](Self::encrypt) output is opaque to callers — adapters
/// embed everything needed to round-trip ([nonce, ciphertext, auth tag])
/// inside the returned bytes so that [`decrypt`](Self::decrypt) needs no
/// extra state.
#[async_trait]
pub trait Cipher: Send + Sync {
    /// Encrypt `plaintext`.  The returned bytes can be safely persisted or
    /// transmitted; their internal layout is an adapter detail.
    async fn encrypt(&self, plaintext: &[u8]) -> AppResult<Vec<u8>>;

    /// Decrypt bytes previously produced by [`encrypt`](Self::encrypt).
    ///
    /// # Errors
    /// Returns [`AppError::internal`](crate::AppError::internal) for
    /// transport / parse failures and
    /// [`AppError::unauthorized`](crate::AppError::unauthorized) when the
    /// authentication tag fails (i.e. the ciphertext was tampered with or
    /// signed with a different key).
    async fn decrypt(&self, ciphertext: &[u8]) -> AppResult<Vec<u8>>;
}
