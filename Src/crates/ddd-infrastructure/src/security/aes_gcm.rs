//! [`Cipher`] adapter using AES-256-GCM with a 96-bit random nonce per call.
//!
//! On-wire layout: `nonce (12 bytes) || ciphertext || tag (16 bytes)`.  The
//! nonce is prepended so [`decrypt`](AesGcmCipher::decrypt) can recover it
//! without extra state.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    AeadCore, Aes256Gcm, Key, Nonce,
};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ddd_shared_kernel::{AppError, AppResult, Cipher};

/// 12-byte (96-bit) GCM nonce length, recommended by NIST SP 800-38D.
const NONCE_LEN: usize = 12;

/// AES-256-GCM [`Cipher`] adapter.
#[derive(Clone)]
pub struct AesGcmCipher {
    cipher: Aes256Gcm,
}

impl std::fmt::Debug for AesGcmCipher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AesGcmCipher").finish_non_exhaustive()
    }
}

impl AesGcmCipher {
    /// Build a cipher from a 32-byte AES-256 key.
    ///
    /// # Errors
    /// Returns [`AppError::internal`] when `key` is not exactly 32 bytes.
    pub fn from_key(key: &[u8]) -> AppResult<Self> {
        if key.len() != 32 {
            return Err(AppError::internal(format!(
                "aes-gcm: expected 32-byte key, got {}",
                key.len()
            )));
        }
        let key = Key::<Aes256Gcm>::from_slice(key);
        Ok(Self { cipher: Aes256Gcm::new(key) })
    }

    /// Build a cipher from a base64-encoded 32-byte key.  Convenient when
    /// the key comes from a config file or env var.
    pub fn from_base64_key(b64: &str) -> AppResult<Self> {
        let bytes = STANDARD
            .decode(b64)
            .map_err(|e| AppError::internal(format!("aes-gcm: invalid base64 key: {e}")))?;
        Self::from_key(&bytes)
    }
}

#[async_trait]
impl Cipher for AesGcmCipher {
    async fn encrypt(&self, plaintext: &[u8]) -> AppResult<Vec<u8>> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| AppError::internal(format!("aes-gcm encrypt: {e}")))?;

        let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        out.extend_from_slice(nonce.as_slice());
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    async fn decrypt(&self, ciphertext: &[u8]) -> AppResult<Vec<u8>> {
        if ciphertext.len() < NONCE_LEN {
            return Err(AppError::internal(
                "aes-gcm decrypt: ciphertext shorter than nonce",
            ));
        }
        let (nonce_bytes, body) = ciphertext.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);
        self.cipher
            .decrypt(nonce, body)
            .map_err(|_| AppError::unauthorized("aes-gcm decrypt: authentication failed"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn round_trip() {
        let key = [7u8; 32];
        let cipher = AesGcmCipher::from_key(&key).unwrap();
        let plaintext = b"hello world";
        let ct = cipher.encrypt(plaintext).await.unwrap();
        assert_ne!(ct.as_slice(), plaintext);
        let pt = cipher.decrypt(&ct).await.unwrap();
        assert_eq!(pt, plaintext);
    }

    #[tokio::test]
    async fn tamper_fails() {
        let cipher = AesGcmCipher::from_key(&[1u8; 32]).unwrap();
        let mut ct = cipher.encrypt(b"data").await.unwrap();
        let last = ct.len() - 1;
        ct[last] ^= 0x01;
        assert!(cipher.decrypt(&ct).await.is_err());
    }
}
