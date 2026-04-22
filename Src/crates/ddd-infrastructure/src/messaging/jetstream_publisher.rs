//! NATS JetStream-backed [`IntegrationEventPublisher`] — at-least-once delivery
//! via a durable server ack.
//!
//! Unlike the core-NATS [`NatsPublisher`](crate::messaging::NatsPublisher)
//! (at-most-once, no persistence), this adapter publishes to a persistent
//! JetStream stream and waits for the server ack before returning.  Each
//! service creates one instance at startup; the stream is auto-created on
//! first use with 7-day file retention.
//!
//! # Example
//!
//! ```no_run
//! # use ddd_infrastructure::messaging::JetStreamPublisher;
//! # async fn run() -> ddd_shared_kernel::AppResult<()> {
//! let publisher =
//!     JetStreamPublisher::connect("nats://localhost:4222", "orders").await?;
//! publisher
//!     .publish_bytes("v1.orders.order.placed", bytes::Bytes::from_static(b"{}"))
//!     .await?;
//! # Ok(()) }
//! ```

use std::time::Duration;

use async_nats::jetstream::{self, stream, Context};
use async_trait::async_trait;
use bytes::Bytes;
use ddd_shared_kernel::{AppError, AppResult, IntegrationEventPublisher};
use serde_json::Value;

/// Default message retention for auto-created JetStream streams (7 days).
pub const DEFAULT_STREAM_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 3600);

/// NATS JetStream publisher providing at-least-once delivery.
///
/// Each outbound subject follows the convention `v1.{service_domain}.{entity}.{action}`.
/// The stream is named `DDD_{SERVICE_DOMAIN_UPPER}` and captures every subject
/// under `v1.{service_domain}.>`.
#[derive(Clone)]
pub struct JetStreamPublisher {
    client: async_nats::Client,
    js: Context,
}

impl JetStreamPublisher {
    /// Connect to NATS, ensure the outbound stream exists, and return a
    /// ready-to-use publisher.
    ///
    /// `service_domain` is the short name used in subjects (e.g. `"orders"`,
    /// `"catalog"`). Returns an error when the NATS connection or stream
    /// provisioning fails.
    pub async fn connect(url: &str, service_domain: &str) -> AppResult<Self> {
        let client = async_nats::connect(url)
            .await
            .map_err(|e| AppError::internal(format!("nats connect failed: {e}")))?;
        let js = jetstream::new(client.clone());
        Self::ensure_stream(&js, service_domain, DEFAULT_STREAM_MAX_AGE).await?;
        tracing::info!(url, domain = service_domain, "JetStreamPublisher connected");
        Ok(Self { client, js })
    }

    /// Wrap an existing NATS client (useful for tests or when the caller
    /// already owns the connection).  Does **not** create a stream — call
    /// [`Self::ensure_stream`] first if needed.
    pub fn from_client(client: async_nats::Client) -> Self {
        let js = jetstream::new(client.clone());
        Self { client, js }
    }

    /// Idempotently create or update the `v1.{service_domain}.>` stream with
    /// file storage and the given retention window.
    pub async fn ensure_stream(
        js: &Context,
        service_domain: &str,
        max_age: Duration,
    ) -> AppResult<()> {
        let name = stream_name_for(service_domain);
        let subject_filter = format!("v1.{service_domain}.>");
        js.get_or_create_stream(stream::Config {
            name: name.clone(),
            subjects: vec![subject_filter],
            storage: stream::StorageType::File,
            max_age,
            ..Default::default()
        })
        .await
        .map_err(|e| AppError::internal(format!("jetstream stream {name}: {e}")))?;
        Ok(())
    }

    /// Publish raw bytes to `subject` and wait for the server ack.
    ///
    /// This is the entry point used by the outbox relay, which reads
    /// pre-serialised JSON from the database.
    pub async fn publish_bytes(&self, subject: &str, payload: Bytes) -> AppResult<()> {
        let ack = self
            .js
            .publish(subject.to_owned(), payload)
            .await
            .map_err(|e| AppError::internal(format!("jetstream publish: {e}")))?;
        ack.await
            .map_err(|e| AppError::internal(format!("jetstream ack: {e}")))?;
        Ok(())
    }

    /// Underlying core NATS client — share it with [`JetStreamSubscriber`] or
    /// use it for request/reply patterns.
    ///
    /// [`JetStreamSubscriber`]: crate::messaging::JetStreamSubscriber
    pub fn client(&self) -> &async_nats::Client {
        &self.client
    }

    /// Underlying JetStream context.
    pub fn context(&self) -> &Context {
        &self.js
    }
}

#[async_trait]
impl IntegrationEventPublisher for JetStreamPublisher {
    async fn publish(&self, subject: &str, payload: &Value) -> AppResult<()> {
        let bytes = serde_json::to_vec(payload)
            .map_err(|e| AppError::serialization(e.to_string()))?;
        self.publish_bytes(subject, bytes.into()).await
    }
}

/// Deterministic stream name from a service-domain string.
///
/// Public so that [`JetStreamSubscriber`](crate::messaging::JetStreamSubscriber)
/// can derive the same name when the subscriber starts before the publisher.
pub fn stream_name_for(service_domain: &str) -> String {
    format!(
        "DDD_{}",
        service_domain.to_uppercase().replace('-', "_")
    )
}
