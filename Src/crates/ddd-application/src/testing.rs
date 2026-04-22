//! In-memory test doubles for the shared-kernel infrastructure ports.
//!
//! Enabled by the `testing` feature (or unconditionally in `#[cfg(test)]`
//! builds).  Downstream service crates can enable the `testing` feature in
//! their `[dev-dependencies]` to use these fakes in integration tests without
//! pulling in a real database or broker.
//!
//! # Example
//! ```rust,ignore
//! use ddd_application::testing::{
//!     InMemoryOutboxRepository, InMemoryIdempotencyStore,
//! };
//! use std::sync::Arc;
//!
//! let outbox = Arc::new(InMemoryOutboxRepository::default());
//! let idempotency = Arc::new(InMemoryIdempotencyStore::default());
//! ```

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ddd_shared_kernel::{
    dead_letter::{DeadLetterMessage, DeadLetterOrigin, DeadLetterRepository},
    idempotency::{IdempotencyRecord, IdempotencyStore},
    inbox::{InboxMessage, InboxRepository},
    outbox::{OutboxMessage, OutboxRepository},
    saga::{SagaInstance, SagaInstanceRepository, SagaStatus},
    AppError, AppResult,
};
use serde_json::Value;
use uuid::Uuid;

// ─── InMemoryOutboxRepository ────────────────────────────────────────────────

/// Thread-safe in-memory implementation of [`OutboxRepository`].
///
/// All operations succeed unless an explicit error is configured. Suitable
/// for unit and integration tests.
#[derive(Debug, Default)]
pub struct InMemoryOutboxRepository {
    messages: Mutex<HashMap<Uuid, OutboxMessage>>,
}

impl InMemoryOutboxRepository {
    /// Return a snapshot of all stored messages (published and unpublished).
    pub fn all(&self) -> Vec<OutboxMessage> {
        self.messages.lock().unwrap().values().cloned().collect()
    }

    /// Return a snapshot of unpublished messages.
    pub fn unpublished(&self) -> Vec<OutboxMessage> {
        self.messages
            .lock()
            .unwrap()
            .values()
            .filter(|m| m.published_at.is_none())
            .cloned()
            .collect()
    }

    /// Count all stored messages.
    pub fn len(&self) -> usize {
        self.messages.lock().unwrap().len()
    }

    /// `true` when no messages have been stored.
    pub fn is_empty(&self) -> bool {
        self.messages.lock().unwrap().is_empty()
    }
}

#[async_trait]
impl OutboxRepository for InMemoryOutboxRepository {
    async fn save(&self, message: &OutboxMessage) -> AppResult<()> {
        self.messages.lock().unwrap().insert(message.id, message.clone());
        Ok(())
    }

    async fn mark_as_published(&self, id: Uuid) -> AppResult<()> {
        let mut guard = self.messages.lock().unwrap();
        let msg = guard
            .get_mut(&id)
            .ok_or_else(|| AppError::not_found("OutboxMessage", id.to_string()))?;
        msg.published_at = Some(Utc::now());
        Ok(())
    }

    async fn mark_as_failed(&self, id: Uuid, error: &str) -> AppResult<()> {
        let mut guard = self.messages.lock().unwrap();
        let msg = guard
            .get_mut(&id)
            .ok_or_else(|| AppError::not_found("OutboxMessage", id.to_string()))?;
        msg.attempts += 1;
        msg.last_error = Some(error.to_owned());
        Ok(())
    }

    async fn find_unpublished(&self, limit: u32) -> AppResult<Vec<OutboxMessage>> {
        let guard = self.messages.lock().unwrap();
        let mut msgs: Vec<_> = guard
            .values()
            .filter(|m| m.published_at.is_none())
            .take(limit as usize)
            .cloned()
            .collect();
        msgs.sort_by_key(|m| m.created_at);
        Ok(msgs)
    }

    async fn delete_published_older_than(&self, older_than: DateTime<Utc>) -> AppResult<u64> {
        let mut guard = self.messages.lock().unwrap();
        let before = guard.len();
        guard.retain(|_, m| m.published_at.is_none_or(|t| t >= older_than));
        Ok((before - guard.len()) as u64)
    }
}

// ─── InMemoryInboxRepository ─────────────────────────────────────────────────

/// Thread-safe in-memory implementation of [`InboxRepository`].
#[derive(Debug, Default)]
pub struct InMemoryInboxRepository {
    messages: Mutex<HashMap<Uuid, InboxMessage>>,
}

impl InMemoryInboxRepository {
    /// Return a snapshot of all stored messages.
    pub fn all(&self) -> Vec<InboxMessage> {
        self.messages.lock().unwrap().values().cloned().collect()
    }

    /// Count all stored messages.
    pub fn len(&self) -> usize {
        self.messages.lock().unwrap().len()
    }

    /// `true` when no messages have been stored.
    pub fn is_empty(&self) -> bool {
        self.messages.lock().unwrap().is_empty()
    }
}

#[async_trait]
impl InboxRepository for InMemoryInboxRepository {
    async fn save(&self, message: &InboxMessage) -> AppResult<bool> {
        let mut guard = self.messages.lock().unwrap();
        if guard.contains_key(&message.id) {
            return Ok(false); // duplicate delivery
        }
        guard.insert(message.id, message.clone());
        Ok(true)
    }

    async fn mark_as_processed(&self, id: Uuid) -> AppResult<()> {
        let mut guard = self.messages.lock().unwrap();
        let msg = guard
            .get_mut(&id)
            .ok_or_else(|| AppError::not_found("InboxMessage", id.to_string()))?;
        msg.processed_at = Some(Utc::now());
        Ok(())
    }

    async fn mark_as_failed(&self, id: Uuid, error: &str) -> AppResult<()> {
        let mut guard = self.messages.lock().unwrap();
        let msg = guard
            .get_mut(&id)
            .ok_or_else(|| AppError::not_found("InboxMessage", id.to_string()))?;
        msg.attempts += 1;
        msg.last_error = Some(error.to_owned());
        Ok(())
    }

    async fn find_unprocessed(&self, limit: u32) -> AppResult<Vec<InboxMessage>> {
        let guard = self.messages.lock().unwrap();
        let mut msgs: Vec<_> = guard
            .values()
            .filter(|m| m.processed_at.is_none())
            .take(limit as usize)
            .cloned()
            .collect();
        msgs.sort_by_key(|m| m.received_at);
        Ok(msgs)
    }

    async fn delete_processed_older_than(&self, older_than: DateTime<Utc>) -> AppResult<u64> {
        let mut guard = self.messages.lock().unwrap();
        let before = guard.len();
        guard.retain(|_, m| m.processed_at.is_none_or(|t| t >= older_than));
        Ok((before - guard.len()) as u64)
    }
}

// ─── InMemoryIdempotencyStore ─────────────────────────────────────────────────

#[derive(Debug)]
struct IdempotencyEntry {
    response: Option<Value>,
    acquired_at: Instant,
    ttl: Duration,
}

/// Thread-safe in-memory implementation of [`IdempotencyStore`].
///
/// TTL-based expiry is enforced on each access (lazy expiry); no background
/// task is required.
#[derive(Debug, Default)]
pub struct InMemoryIdempotencyStore {
    entries: Mutex<HashMap<String, IdempotencyEntry>>,
}

impl InMemoryIdempotencyStore {
    /// Count currently held (non-expired) keys.
    pub fn len(&self) -> usize {
        let now = Instant::now();
        self.entries
            .lock()
            .unwrap()
            .values()
            .filter(|e| now.duration_since(e.acquired_at) < e.ttl)
            .count()
    }

    /// `true` when no (non-expired) keys are held.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl IdempotencyStore for InMemoryIdempotencyStore {
    async fn try_acquire(&self, key: &str, ttl: Duration) -> AppResult<bool> {
        let mut guard = self.entries.lock().unwrap();
        let now = Instant::now();
        // Purge expired entries while we hold the lock.
        guard.retain(|_, e| now.duration_since(e.acquired_at) < e.ttl);

        if guard.contains_key(key) {
            return Ok(false);
        }
        guard.insert(
            key.to_owned(),
            IdempotencyEntry { response: None, acquired_at: now, ttl },
        );
        Ok(true)
    }

    async fn store_response(&self, key: &str, response: &Value) -> AppResult<()> {
        let mut guard = self.entries.lock().unwrap();
        if let Some(entry) = guard.get_mut(key) {
            entry.response = Some(response.clone());
        }
        Ok(())
    }

    async fn get_response(&self, key: &str) -> AppResult<Option<IdempotencyRecord>> {
        let guard = self.entries.lock().unwrap();
        let Some(entry) = guard.get(key) else {
            return Ok(None);
        };
        if Instant::now().duration_since(entry.acquired_at) >= entry.ttl {
            return Ok(None);
        }
        Ok(entry.response.as_ref().map(|r| IdempotencyRecord {
            key: key.to_owned(),
            response: r.clone(),
        }))
    }

    async fn release(&self, key: &str) -> AppResult<()> {
        self.entries.lock().unwrap().remove(key);
        Ok(())
    }
}

// ─── InMemoryDeadLetterRepository ────────────────────────────────────────────

/// Thread-safe in-memory implementation of [`DeadLetterRepository`].
#[derive(Debug, Default)]
pub struct InMemoryDeadLetterRepository {
    messages: Mutex<Vec<DeadLetterMessage>>,
}

impl InMemoryDeadLetterRepository {
    /// Return a snapshot of all dead-letter messages.
    pub fn all(&self) -> Vec<DeadLetterMessage> {
        self.messages.lock().unwrap().clone()
    }

    /// Count all stored dead-letter messages.
    pub fn len(&self) -> usize {
        self.messages.lock().unwrap().len()
    }

    /// `true` when no dead-letter messages have been stored.
    pub fn is_empty(&self) -> bool {
        self.messages.lock().unwrap().is_empty()
    }
}

#[async_trait]
impl DeadLetterRepository for InMemoryDeadLetterRepository {
    async fn save(&self, message: &DeadLetterMessage) -> AppResult<()> {
        self.messages.lock().unwrap().push(message.clone());
        Ok(())
    }

    async fn find_by_origin(
        &self,
        origin: DeadLetterOrigin,
        limit: u32,
    ) -> AppResult<Vec<DeadLetterMessage>> {
        let guard = self.messages.lock().unwrap();
        let mut msgs: Vec<_> = guard
            .iter()
            .filter(|m| m.origin == origin)
            .take(limit as usize)
            .cloned()
            .collect();
        msgs.sort_by(|a, b| b.dead_lettered_at.cmp(&a.dead_lettered_at));
        Ok(msgs)
    }

    async fn delete_older_than(&self, older_than: DateTime<Utc>) -> AppResult<u64> {
        let mut guard = self.messages.lock().unwrap();
        let before = guard.len();
        guard.retain(|m| m.dead_lettered_at >= older_than);
        Ok((before - guard.len()) as u64)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_outbox_msg(id: Uuid) -> OutboxMessage {
        OutboxMessage {
            id,
            aggregate_id: "agg-1".into(),
            aggregate_type: "Order".into(),
            event_type: "order.created.v1".into(),
            subject: "orders.created".into(),
            payload: json!({"order_id": "agg-1"}),
            created_at: Utc::now(),
            published_at: None,
            attempts: 0,
            last_error: None,
        }
    }

    fn make_inbox_msg(id: Uuid) -> InboxMessage {
        InboxMessage::new(id, "order.created.v1", "orders.created", json!({}), "order-service")
    }

    // ── OutboxRepository ──────────────────────────────────────────────────

    #[tokio::test]
    async fn outbox_save_and_find_unpublished() {
        let repo = InMemoryOutboxRepository::default();
        let id = Uuid::now_v7();
        repo.save(&make_outbox_msg(id)).await.unwrap();

        let unpublished = repo.find_unpublished(10).await.unwrap();
        assert_eq!(unpublished.len(), 1);
        assert_eq!(unpublished[0].id, id);
    }

    #[tokio::test]
    async fn outbox_mark_published() {
        let repo = InMemoryOutboxRepository::default();
        let id = Uuid::now_v7();
        repo.save(&make_outbox_msg(id)).await.unwrap();
        repo.mark_as_published(id).await.unwrap();

        let unpublished = repo.find_unpublished(10).await.unwrap();
        assert!(unpublished.is_empty());
    }

    #[tokio::test]
    async fn outbox_mark_failed_increments_attempts() {
        let repo = InMemoryOutboxRepository::default();
        let id = Uuid::now_v7();
        repo.save(&make_outbox_msg(id)).await.unwrap();
        repo.mark_as_failed(id, "connection refused").await.unwrap();

        let msgs = repo.all();
        assert_eq!(msgs[0].attempts, 1);
        assert_eq!(msgs[0].last_error.as_deref(), Some("connection refused"));
    }

    #[tokio::test]
    async fn outbox_delete_published_older_than() {
        let repo = InMemoryOutboxRepository::default();
        let id = Uuid::now_v7();
        repo.save(&make_outbox_msg(id)).await.unwrap();
        repo.mark_as_published(id).await.unwrap();

        // Delete messages published before "now + 1s" — should remove our message.
        let deleted = repo
            .delete_published_older_than(Utc::now() + chrono::Duration::seconds(1))
            .await
            .unwrap();
        assert_eq!(deleted, 1);
        assert!(repo.is_empty());
    }

    // ── InboxRepository ───────────────────────────────────────────────────

    #[tokio::test]
    async fn inbox_deduplicates() {
        let repo = InMemoryInboxRepository::default();
        let id = Uuid::now_v7();
        let first = repo.save(&make_inbox_msg(id)).await.unwrap();
        let second = repo.save(&make_inbox_msg(id)).await.unwrap();
        assert!(first, "first insert should succeed");
        assert!(!second, "duplicate should return false");
        assert_eq!(repo.len(), 1);
    }

    #[tokio::test]
    async fn inbox_mark_processed() {
        let repo = InMemoryInboxRepository::default();
        let id = Uuid::now_v7();
        repo.save(&make_inbox_msg(id)).await.unwrap();
        repo.mark_as_processed(id).await.unwrap();

        let unprocessed = repo.find_unprocessed(10).await.unwrap();
        assert!(unprocessed.is_empty());
    }

    // ── IdempotencyStore ──────────────────────────────────────────────────

    #[tokio::test]
    async fn idempotency_try_acquire_once() {
        let store = InMemoryIdempotencyStore::default();
        let acquired = store.try_acquire("key-1", Duration::from_secs(60)).await.unwrap();
        assert!(acquired);
        let second = store.try_acquire("key-1", Duration::from_secs(60)).await.unwrap();
        assert!(!second, "duplicate key must return false");
    }

    #[tokio::test]
    async fn idempotency_store_and_get_response() {
        let store = InMemoryIdempotencyStore::default();
        store.try_acquire("key-2", Duration::from_secs(60)).await.unwrap();
        store.store_response("key-2", &json!({"id": "abc"})).await.unwrap();

        let record = store.get_response("key-2").await.unwrap();
        assert!(record.is_some());
        assert_eq!(record.unwrap().response, json!({"id": "abc"}));
    }

    #[tokio::test]
    async fn idempotency_release() {
        let store = InMemoryIdempotencyStore::default();
        store.try_acquire("key-3", Duration::from_secs(60)).await.unwrap();
        store.release("key-3").await.unwrap();

        let reacquired = store.try_acquire("key-3", Duration::from_secs(60)).await.unwrap();
        assert!(reacquired, "released key should be re-acquirable");
    }

    // ── DeadLetterRepository ──────────────────────────────────────────────

    #[tokio::test]
    async fn dead_letter_save_and_find_by_origin() {
        use ddd_shared_kernel::dead_letter::{DeadLetterMessage, DeadLetterOrigin};

        let repo = InMemoryDeadLetterRepository::default();
        let dl = DeadLetterMessage::new(
            Uuid::now_v7(),
            DeadLetterOrigin::Outbox,
            "order.created.v1",
            "orders.created",
            json!({}),
            5,
            "too many retries",
            Utc::now(),
        );
        repo.save(&dl).await.unwrap();
        assert_eq!(repo.len(), 1);

        let found = repo.find_by_origin(DeadLetterOrigin::Outbox, 10).await.unwrap();
        assert_eq!(found.len(), 1);

        let none = repo.find_by_origin(DeadLetterOrigin::Inbox, 10).await.unwrap();
        assert!(none.is_empty());
    }
}

// ─── InMemorySagaInstanceRepository ──────────────────────────────────────────

/// Thread-safe in-memory implementation of [`SagaInstanceRepository`].
///
/// Suitable for unit and integration tests. All operations succeed unless the
/// id is not found (for `update` / `find_by_id`).
#[derive(Debug, Default)]
pub struct InMemorySagaInstanceRepository {
    instances: Mutex<HashMap<Uuid, SagaInstance>>,
}

impl InMemorySagaInstanceRepository {
    /// Return a snapshot of all stored instances.
    pub fn all(&self) -> Vec<SagaInstance> {
        self.instances.lock().unwrap().values().cloned().collect()
    }

    /// Number of stored instances.
    pub fn len(&self) -> usize {
        self.instances.lock().unwrap().len()
    }

    /// True when no instances are stored.
    pub fn is_empty(&self) -> bool {
        self.instances.lock().unwrap().is_empty()
    }
}

#[async_trait]
impl SagaInstanceRepository for InMemorySagaInstanceRepository {
    async fn save(&self, instance: &SagaInstance) -> AppResult<()> {
        self.instances
            .lock()
            .unwrap()
            .insert(instance.id, instance.clone());
        Ok(())
    }

    async fn update(&self, instance: &SagaInstance) -> AppResult<()> {
        let mut store = self.instances.lock().unwrap();
        let existing = store.get(&instance.id).ok_or_else(|| AppError::NotFound {
            resource: "SagaInstance".into(),
            id: instance.id.to_string(),
        })?;
        if existing.version >= instance.version {
            return Err(AppError::Conflict {
                message: "version mismatch".into(),
            });
        }
        store.insert(instance.id, instance.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> AppResult<SagaInstance> {
        self.instances
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| AppError::NotFound {
                resource: "SagaInstance".into(),
                id: id.to_string(),
            })
    }

    async fn find_by_status(&self, status: SagaStatus) -> AppResult<Vec<SagaInstance>> {
        Ok(self
            .instances
            .lock()
            .unwrap()
            .values()
            .filter(|s| s.status == status)
            .cloned()
            .collect())
    }
}
