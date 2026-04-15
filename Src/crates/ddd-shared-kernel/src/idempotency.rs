//! Idempotency-key support.
//!
//! Commands (e.g. `CreateOrder`) may be retried by clients, causing
//! duplicates. An [`IdempotencyStore`] records the key on first execution
//! and rejects (or returns the cached response) on subsequent attempts.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::AppResult;

// ─── IdempotencyRecord ──────────────────────────────────────────────────────

/// The stored result of a previously-executed command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyRecord {
    /// The idempotency key.
    pub key: String,
    /// Serialised response from the original execution.
    pub response: Value,
}

// ─── IdempotencyStore ───────────────────────────────────────────────────────

/// Port for persisting idempotency keys.
///
/// Implementations may use a database table, Redis `SET NX EX`, or any store
/// that supports atomic "insert if absent" semantics.
#[async_trait]
pub trait IdempotencyStore: Send + Sync {
    /// Try to claim `key`. Returns `true` when the key was freshly inserted
    /// (first request), `false` when it already existed (duplicate).
    ///
    /// `ttl` controls how long the key is retained; after expiry the same key
    /// can be reused.
    async fn try_acquire(&self, key: &str, ttl: Duration) -> AppResult<bool>;

    /// Store the serialised response for a previously-acquired key.
    ///
    /// Called after the command handler has executed successfully.
    async fn store_response(&self, key: &str, response: &Value) -> AppResult<()>;

    /// Retrieve the cached response for a key, if one exists.
    async fn get_response(&self, key: &str) -> AppResult<Option<IdempotencyRecord>>;

    /// Remove a key (e.g. after a failed execution so the client can retry).
    async fn release(&self, key: &str) -> AppResult<()>;
}
