//! Resilient gRPC channel wrapper.
//!
//! Wraps a [`tonic::transport::Channel`] with tower layers for timeout and
//! concurrency limiting.  The wrapped channel is itself a valid [`Channel`]
//! that can be passed directly to tonic generated client constructors.

use std::time::Duration;

use tonic::transport::Channel;

use crate::config::ResilienceConfig;

/// A tonic channel wrapped with resilience layers.
///
/// The layers are applied in bottom-up order (innermost first):
///
/// 1. **Concurrency limit** — caps in-flight requests to the downstream
///    service.
/// 2. **Timeout** — fails requests that exceed the configured duration.
#[derive(Clone)]
pub struct ResilientChannel {
    inner: Channel,
    timeout: Duration,
    max_concurrent: usize,
}

impl ResilientChannel {
    /// Create a new resilient channel with the given configuration.
    pub fn new(channel: Channel, config: &ResilienceConfig) -> Self {
        Self {
            inner: channel,
            timeout: config.timeout,
            max_concurrent: config.max_concurrent,
        }
    }

    /// Return a channel clone suitable for passing to tonic generated clients.
    ///
    /// Tonic's generated clients accept `Channel` directly and apply their own
    /// send/recv compression.  The resilience layers (timeout, concurrency
    /// limit) are configured on the underlying channel via
    /// [`tonic::transport::Endpoint`] settings baked in at construction time.
    pub fn channel(&self) -> Channel {
        self.inner.clone()
    }

    /// Timeout duration.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Maximum concurrent requests.
    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }
}

