//! gRPC client connection pool.
//!
//! Generic pool keyed by service name. Each entry is a [`ResilientChannel`]
//! built from a tonic [`Channel`] and the shared [`ResilienceConfig`].
//! Channels are constructed lazily (`connect_lazy`) so the pool never
//! blocks on startup.

use std::collections::HashMap;
use std::sync::Arc;

use ddd_shared_kernel::{AppError, AppResult};
use tonic::transport::Channel;

use crate::config::ResilienceConfig;

use super::resilient_client::ResilientChannel;

/// Manages tonic [`Channel`] instances keyed by service name.
#[derive(Clone, Default)]
pub struct GrpcClientPool {
    channels: Arc<HashMap<String, ResilientChannel>>,
}

impl GrpcClientPool {
    /// Build a pool from an iterator of `(service_name, url)` pairs.
    ///
    /// All channels share the same [`ResilienceConfig`]. Use
    /// [`GrpcClientPool::builder`] when each upstream needs its own
    /// timeout / concurrency tuning.
    pub fn from_services<I, K, V>(services: I, resilience: &ResilienceConfig) -> AppResult<Self>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: AsRef<str>,
    {
        let mut builder = Self::builder();
        for (name, url) in services {
            let name = name.into();
            builder = builder.add(name, url.as_ref(), resilience.clone())?;
        }
        Ok(builder.build())
    }

    /// Start a builder for per-upstream resilience tuning.
    pub fn builder() -> GrpcClientPoolBuilder {
        GrpcClientPoolBuilder::default()
    }

    /// Look up the resilient channel for a downstream service.
    pub fn get(&self, service: &str) -> AppResult<&ResilientChannel> {
        self.channels
            .get(service)
            .ok_or_else(|| AppError::internal(format!("unknown service: {service}")))
    }

    /// Return the raw tonic [`Channel`] for a service.
    pub fn channel(&self, service: &str) -> AppResult<Channel> {
        Ok(self.get(service)?.channel())
    }

    /// Number of registered services.
    pub fn len(&self) -> usize {
        self.channels.len()
    }

    /// True when no services are registered.
    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }
}

/// Incremental builder for [`GrpcClientPool`].
#[derive(Default)]
pub struct GrpcClientPoolBuilder {
    channels: HashMap<String, ResilientChannel>,
}

impl GrpcClientPoolBuilder {
    /// Add a service to the pool. Last write wins for a given name.
    pub fn add(
        mut self,
        name: impl Into<String>,
        url: &str,
        resilience: ResilienceConfig,
    ) -> AppResult<Self> {
        let name = name.into();
        let channel = Channel::from_shared(url.to_owned())
            .map_err(|e| AppError::internal(format!("invalid URL for {name}: {e}")))?
            .connect_lazy();
        let resilient = ResilientChannel::new(channel, &resilience);
        tracing::info!(service = %name, url = %url, "registered gRPC channel");
        self.channels.insert(name, resilient);
        Ok(self)
    }

    /// Finalise the pool.
    pub fn build(self) -> GrpcClientPool {
        GrpcClientPool {
            channels: Arc::new(self.channels),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn builder_round_trips() {
        let resilience = ResilienceConfig::default();
        let pool = GrpcClientPool::from_services(
            [
                ("orders", "http://127.0.0.1:50051"),
                ("users", "http://127.0.0.1:50052"),
            ],
            &resilience,
        )
        .expect("pool");

        assert_eq!(pool.len(), 2);
        pool.channel("orders").expect("orders channel");
        pool.channel("users").expect("users channel");
        assert!(pool.channel("missing").is_err());
    }}
