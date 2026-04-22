//! Generic cache port (secondary port / driven adapter).
//!
//! [`Cache`] is the byte-level contract every cache adapter must implement;
//! the typed helpers ([`CacheExt::get`], [`CacheExt::set`],
//! [`CacheExt::get_or_set`]) layer JSON (de)serialisation on top so callers
//! can store domain DTOs directly.
//!
//! Concrete adapters — Redis today — live in `ddd-infrastructure` behind
//! the `cache` feature.
//!
//! # Best-effort vs strict semantics
//!
//! Adapters typically default to **best-effort**: a cache miss caused by a
//! transport error returns `Ok(None)` so the surrounding handler falls
//! through to the upstream service exactly as it would on a normal miss.
//! Adapters that need fail-fast behaviour expose an opt-in `strict`
//! constructor that surfaces transport errors as
//! [`AppError::internal`](crate::AppError::internal).

use std::future::Future;
use std::time::Duration;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

use crate::{AppError, AppResult};

/// Byte-level cache port.
#[async_trait]
pub trait Cache: Send + Sync {
    /// Read raw bytes for `key`, or `None` on miss.
    async fn get_raw(&self, key: &str) -> AppResult<Option<Vec<u8>>>;

    /// Store `value` at `key` with the given `ttl`.
    async fn set_raw(&self, key: &str, value: &[u8], ttl: Duration) -> AppResult<()>;

    /// Delete `key`.  Idempotent — deleting a missing key succeeds.
    async fn delete(&self, key: &str) -> AppResult<()>;
}

/// JSON-typed convenience methods layered on top of [`Cache`].
///
/// Implemented for every `T: Cache + ?Sized`, so it works through both
/// concrete adapters and `Arc<dyn Cache>`.
#[async_trait]
pub trait CacheExt: Cache {
    /// Fetch and JSON-deserialise the value at `key`.
    async fn get<T>(&self, key: &str) -> AppResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        match self.get_raw(key).await? {
            None => Ok(None),
            Some(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| AppError::internal(format!("cache decode '{key}': {e}"))),
        }
    }

    /// JSON-serialise `value` and store it at `key` with the given `ttl`.
    async fn set<T>(&self, key: &str, value: &T, ttl: Duration) -> AppResult<()>
    where
        T: Serialize + Sync,
    {
        let bytes = serde_json::to_vec(value)
            .map_err(|e| AppError::internal(format!("cache encode '{key}': {e}")))?;
        self.set_raw(key, &bytes, ttl).await
    }

    /// Read-through helper: return the cached value if present, otherwise
    /// invoke `f`, store the result, and return it.
    ///
    /// `f` is only awaited on a miss; cache writes are best-effort and
    /// never fail the call.
    async fn get_or_set<T, F, Fut>(&self, key: &str, ttl: Duration, f: F) -> AppResult<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = AppResult<T>> + Send,
    {
        if let Some(hit) = self.get::<T>(key).await? {
            return Ok(hit);
        }
        let value = f().await?;
        // best-effort: a write failure must not fail the call
        let _ = self.set(key, &value, ttl).await;
        Ok(value)
    }
}

impl<T: Cache + ?Sized> CacheExt for T {}
