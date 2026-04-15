//! NATS-backed [`IntegrationEventPublisher`].

use std::collections::HashMap;

use async_trait::async_trait;
use ddd_shared_kernel::{AppError, AppResult, IntegrationEventPublisher};
use serde_json::Value;

/// Publishes integration events to a NATS subject.
#[derive(Clone)]
pub struct NatsPublisher {
    client: async_nats::Client,
}

impl NatsPublisher {
    /// Wrap an existing NATS client.
    pub fn new(client: async_nats::Client) -> Self {
        Self { client }
    }

    /// Publish with per-message headers.
    pub async fn publish_with_headers(
        &self,
        subject: &str,
        headers: HashMap<String, String>,
        payload: &Value,
    ) -> AppResult<()> {
        let bytes = serde_json::to_vec(payload)
            .map_err(|e| AppError::serialization(e.to_string()))?;

        let mut hdr = async_nats::HeaderMap::new();
        for (k, v) in headers {
            hdr.insert(k.as_str(), v.as_str());
        }

        self.client
            .publish_with_headers(subject.to_owned(), hdr, bytes.into())
            .await
            .map_err(|e| AppError::internal(format!("nats publish failed: {e}")))?;
        Ok(())
    }
}

#[async_trait]
impl IntegrationEventPublisher for NatsPublisher {
    async fn publish(&self, subject: &str, payload: &Value) -> AppResult<()> {
        let bytes = serde_json::to_vec(payload)
            .map_err(|e| AppError::serialization(e.to_string()))?;
        self.client
            .publish(subject.to_owned(), bytes.into())
            .await
            .map_err(|e| AppError::internal(format!("nats publish failed: {e}")))?;
        Ok(())
    }
}
