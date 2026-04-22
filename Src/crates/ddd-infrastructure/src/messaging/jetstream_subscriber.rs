//! NATS JetStream durable pull-subscriber that writes each message to the
//! inbox for idempotent downstream processing.
//!
//! JetStream guarantees at-least-once delivery — the same message can arrive
//! more than once (e.g. if the service crashes between processing and ack,
//! or if JetStream redelivers an unacked message). Dedup is performed by
//! the inbox: [`InboxRepository::save`] returns `false` when the message id
//! already exists, and we ack-and-skip in that case.
//!
//! The dedup key is a v5 UUID derived from `"{stream}:{stream_sequence}"` so
//! retries of the same physical message always hash to the same id. When the
//! publisher includes a `message-id` header we honour it instead.
//!
//! # Example
//!
//! ```no_run
//! # use std::sync::Arc;
//! # use std::time::Duration;
//! # use async_nats::jetstream;
//! # use ddd_infrastructure::messaging::{JetStreamSubscriber, JetStreamSubscriberConfig};
//! # use ddd_shared_kernel::InboxRepository;
//! # async fn run<R: InboxRepository + 'static>(
//! #     client: async_nats::Client,
//! #     inbox: Arc<R>,
//! # ) -> ddd_shared_kernel::AppResult<()> {
//! let js = jetstream::new(client);
//! let subscriber = JetStreamSubscriber::new(
//!     js,
//!     inbox,
//!     JetStreamSubscriberConfig {
//!         service_domain: "orders".into(),
//!         subject: "v1.orders.order.placed".into(),
//!         consumer_group: "inventory-service".into(),
//!         ..Default::default()
//!     },
//! );
//! tokio::spawn(subscriber.start());
//! # Ok(()) }
//! ```

use std::sync::Arc;
use std::time::Duration;

use async_nats::jetstream::{
    consumer::{pull, AckPolicy},
    stream, Context,
};
use ddd_shared_kernel::{AppError, AppResult, InboxMessage, InboxRepository};
use futures::StreamExt;
use serde_json::Value;
use uuid::Uuid;

use crate::messaging::jetstream_publisher::{stream_name_for, DEFAULT_STREAM_MAX_AGE};

/// Default ack-wait before JetStream considers the message un-acked and
/// redelivers it (30 s).
pub const DEFAULT_ACK_WAIT: Duration = Duration::from_secs(30);

/// Default maximum redelivery attempts before the message stops being
/// redelivered (and the caller's DLQ logic must take over).
pub const DEFAULT_MAX_DELIVER: i64 = 10;

/// Configuration for a [`JetStreamSubscriber`].
#[derive(Debug, Clone)]
pub struct JetStreamSubscriberConfig {
    /// Service-domain segment of the subject (e.g. `"orders"`).  Used to
    /// derive the stream name; must match the publisher's `service_domain`.
    pub service_domain: String,
    /// Exact subject to filter on (e.g. `"v1.orders.order.placed"` or a
    /// wildcard like `"v1.orders.order.*"`).
    pub subject: String,
    /// Durable consumer name — one per logical consumer group.  Multiple
    /// replicas using the same `consumer_group` share work; different
    /// groups receive independent copies of each message.
    pub consumer_group: String,
    /// Redeliver messages whose ack does not arrive within this window.
    pub ack_wait: Duration,
    /// Maximum redelivery attempts for the same physical message.
    pub max_deliver: i64,
    /// Retention window used when auto-creating the stream.
    pub stream_max_age: Duration,
    /// Source-service label recorded on each [`InboxMessage`] when the
    /// publisher did not provide a `source` header.
    pub default_source: String,
}

impl Default for JetStreamSubscriberConfig {
    fn default() -> Self {
        Self {
            service_domain: String::new(),
            subject: String::new(),
            consumer_group: String::new(),
            ack_wait: DEFAULT_ACK_WAIT,
            max_deliver: DEFAULT_MAX_DELIVER,
            stream_max_age: DEFAULT_STREAM_MAX_AGE,
            default_source: String::new(),
        }
    }
}

/// Durable JetStream pull subscriber with inbox-backed dedup.
pub struct JetStreamSubscriber<I: InboxRepository> {
    js: Context,
    inbox_repo: Arc<I>,
    config: JetStreamSubscriberConfig,
}

impl<I: InboxRepository + 'static> JetStreamSubscriber<I> {
    /// Build a subscriber. Does not connect yet — call [`Self::start`].
    pub fn new(js: Context, inbox_repo: Arc<I>, config: JetStreamSubscriberConfig) -> Self {
        Self { js, inbox_repo, config }
    }

    /// Run the subscription loop until the JetStream message stream closes.
    ///
    /// Ensures the source stream and the durable consumer both exist (so the
    /// subscriber can start before the publisher) and then drains messages
    /// forever, acking each one after a successful inbox write.
    pub async fn start(self) -> AppResult<()> {
        let stream_name = stream_name_for(&self.config.service_domain);
        let subject_filter = format!("v1.{}.>", self.config.service_domain);

        let stream = self
            .js
            .get_or_create_stream(stream::Config {
                name: stream_name.clone(),
                subjects: vec![subject_filter],
                storage: stream::StorageType::File,
                max_age: self.config.stream_max_age,
                ..Default::default()
            })
            .await
            .map_err(|e| AppError::internal(format!("jetstream stream {stream_name}: {e}")))?;

        let consumer_name = format!(
            "{}--{}",
            self.config.consumer_group,
            self.config.subject.replace('.', "-")
        );

        let consumer = stream
            .get_or_create_consumer(
                &consumer_name,
                pull::Config {
                    durable_name: Some(consumer_name.clone()),
                    filter_subject: self.config.subject.clone(),
                    ack_policy: AckPolicy::Explicit,
                    ack_wait: self.config.ack_wait,
                    max_deliver: self.config.max_deliver,
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| {
                AppError::internal(format!("jetstream consumer {consumer_name}: {e}"))
            })?;

        tracing::info!(
            subject = %self.config.subject,
            consumer = %consumer_name,
            "JetStreamSubscriber started"
        );

        let mut messages = consumer
            .messages()
            .await
            .map_err(|e| AppError::internal(format!("jetstream messages: {e}")))?;

        while let Some(item) = messages.next().await {
            match item {
                Ok(msg) => self.handle_message(msg).await,
                Err(e) => {
                    tracing::warn!(error = %e, "JetStreamSubscriber: stream error — continuing");
                }
            }
        }

        tracing::info!(subject = %self.config.subject, "JetStreamSubscriber stopped");
        Ok(())
    }

    async fn handle_message(&self, msg: async_nats::jetstream::Message) {
        let id = derive_message_id(&msg, &msg.subject);
        let payload: Value =
            serde_json::from_slice(&msg.payload).unwrap_or(Value::Null);
        let event_type = header_value(&msg, "event-type").unwrap_or_default();
        let source = header_value(&msg, "source")
            .unwrap_or_else(|| self.config.default_source.clone());

        let inbox_msg =
            InboxMessage::new(id, event_type, msg.subject.as_str(), payload, source);

        match self.inbox_repo.save(&inbox_msg).await {
            Ok(true) => {
                if let Err(e) = msg.ack().await {
                    tracing::warn!(
                        message_id = %id,
                        error = %e,
                        "JetStreamSubscriber: ack failed after save"
                    );
                }
            }
            Ok(false) => {
                // Duplicate delivery — already in the inbox. Ack to stop
                // redelivery; the existing inbox row is still processed by
                // the InboxProcessor.
                tracing::debug!(
                    message_id = %id,
                    subject = %msg.subject,
                    "JetStreamSubscriber: duplicate delivery, ack-skipping"
                );
                let _ = msg.ack().await;
            }
            Err(e) => {
                // Inbox write failed — do not ack, let JetStream redeliver.
                tracing::warn!(
                    message_id = %id,
                    subject = %msg.subject,
                    error = %e,
                    "JetStreamSubscriber: inbox save failed — will redeliver"
                );
            }
        }
    }
}

fn header_value(msg: &async_nats::jetstream::Message, key: &str) -> Option<String> {
    msg.headers
        .as_ref()
        .and_then(|h| h.get(key))
        .map(|v| v.to_string())
}

/// Derive a stable [`Uuid`] for an inbound JetStream message.
///
/// Preference order:
/// 1. The publisher-supplied `message-id` header, if it parses as a UUID.
/// 2. A v5 UUID hashed from `"{stream}:{stream_sequence}"` — stable across
///    redeliveries of the same physical message.
/// 3. A random v7 UUID (fallback when JetStream metadata is missing, which
///    should never happen for a durable consumer).
fn derive_message_id(msg: &async_nats::jetstream::Message, subject: &str) -> Uuid {
    if let Some(raw) = header_value(msg, "message-id") {
        if let Ok(id) = Uuid::parse_str(&raw) {
            return id;
        }
    }
    if let Ok(info) = msg.info() {
        let key = format!("{}:{}", info.stream, info.stream_sequence);
        return Uuid::new_v5(&Uuid::NAMESPACE_OID, key.as_bytes());
    }
    // Last resort: random id — breaks dedup guarantees but keeps the pipe
    // moving. Very rare in practice (JetStream always populates info()).
    tracing::warn!(subject, "JetStreamSubscriber: missing message info, using random id");
    Uuid::now_v7()
}
