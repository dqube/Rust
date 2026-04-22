//! Cryptographic adapters for the [`Hasher`] and [`Cipher`] ports.
//!
//! Enabled with the `crypto` feature.  Currently ships PBKDF2-SHA256
//! ([`Pbkdf2Hasher`]) and AES-256-GCM ([`AesGcmCipher`]).

pub mod aes_gcm;
pub mod pbkdf2;

pub use aes_gcm::AesGcmCipher;
pub use pbkdf2::Pbkdf2Hasher;
