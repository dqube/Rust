//! [`Cache`] adapter backed by Redis using a pooled multiplexed
//! connection.
//!
//! Two semantic modes:
//!
//! * **best-effort** ([`RedisCache::connect`]) — the default.  Read or
//!   write failures are logged and turned into a clean miss / no-op so the
//!   surrounding handler falls through to upstream.  Matches the behaviour
//!   of the old BFF cache.
//! * **strict** ([`RedisCache::connect_strict`]) — every transport error
//!   surfaces as [`AppError::internal`].  Useful when the cache is on the
//!   critical path (e.g. rate-limit counters).

use std::time::Duration;

use async_trait::async_trait;
use ddd_shared_kernel::{AppError, AppResult, Cache};
use redis::{aio::MultiplexedConnection, AsyncCommands, Client};
use tracing::{debug, warn};

/// Redis-backed [`Cache`].
#[derive(Clone)]
pub struct RedisCache {
    conn: MultiplexedConnection,
    prefix: String,
    strict: bool,
}

impl std::fmt::Debug for RedisCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisCache")
            .field("prefix", &self.prefix)
            .field("strict", &self.strict)
            .finish_non_exhaustive()
    }
}

impl RedisCache {
    /// Connect with **best-effort** semantics.  `prefix` is prepended (with
    /// a `:` separator) to every key — services typically pass their own
    /// short name (`"adminbff"`, `"posbff"`).
    pub async fn connect(url: &str, prefix: impl Into<String>) -> AppResult<Self> {
        Self::open(url, prefix, false).await
    }

    /// Connect with **strict** semantics — transport errors propagate as
    /// [`AppError::internal`].
    pub async fn connect_strict(url: &str, prefix: impl Into<String>) -> AppResult<Self> {
        Self::open(url, prefix, true).await
    }

    async fn open(url: &str, prefix: impl Into<String>, strict: bool) -> AppResult<Self> {
        let client = Client::open(url)
            .map_err(|e| AppError::internal(format!("redis client open: {e}")))?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::internal(format!("redis connect: {e}")))?;
        Ok(Self { conn, prefix: prefix.into(), strict })
    }

    fn key(&self, k: &str) -> String {
        if self.prefix.is_empty() {
            k.to_owned()
        } else {
            format!("{}:{}", self.prefix, k)
        }
    }

    /// Convert a transport error into either an `AppError` (strict) or
    /// `Ok(None)` / `Ok(())` (best-effort, after logging).
    fn handle_err<T>(&self, op: &str, err: redis::RedisError, fallback: T) -> AppResult<T> {
        if self.strict {
            Err(AppError::internal(format!("redis {op}: {err}")))
        } else {
            warn!(target: "ddd_infrastructure::cache", error = %err, "redis {op} failed; falling back");
            Ok(fallback)
        }
    }
}

#[async_trait]
impl Cache for RedisCache {
    async fn get_raw(&self, key: &str) -> AppResult<Option<Vec<u8>>> {
        let mut conn = self.conn.clone();
        match conn.get::<_, Option<Vec<u8>>>(self.key(key)).await {
            Ok(v) => {
                debug!(target: "ddd_infrastructure::cache", key, hit = v.is_some(), "redis get");
                Ok(v)
            }
            Err(e) => self.handle_err("get", e, None),
        }
    }

    async fn set_raw(&self, key: &str, value: &[u8], ttl: Duration) -> AppResult<()> {
        let mut conn = self.conn.clone();
        let res: redis::RedisResult<()> = conn
            .set_ex(self.key(key), value, ttl.as_secs().max(1))
            .await;
        match res {
            Ok(()) => Ok(()),
            Err(e) => self.handle_err("set", e, ()),
        }
    }

    async fn delete(&self, key: &str) -> AppResult<()> {
        let mut conn = self.conn.clone();
        let res: redis::RedisResult<()> = conn.del(self.key(key)).await;
        match res {
            Ok(()) => Ok(()),
            Err(e) => self.handle_err("del", e, ()),
        }
    }
}
