//! Upstream channel registry.
//!
//! Builds a [`tonic::transport::Channel`] per upstream declared in
//! `routes.yaml`. Channels are lazy (`connect_lazy`) — the first gRPC call
//! establishes the underlying HTTP/2 connection. Per-request timeouts are
//! applied by the edge service layer rather than baked into the channel so
//! the descriptor-driven transcoder stays in control of the deadline.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use ddd_shared_kernel::{AppError, AppResult};
use tonic::transport::{Channel, Endpoint};

use super::route_config::UpstreamConfig;

/// Single upstream entry.
#[derive(Clone)]
pub struct Upstream {
    /// Upstream name (as declared in `routes.yaml`).
    pub name: String,
    /// gRPC channel (lazy — first call connects).
    pub channel: Channel,
    /// Per-request timeout applied by the edge when calling this upstream.
    pub timeout: Duration,
}

/// Name → upstream map.
#[derive(Clone)]
pub struct UpstreamRegistry {
    inner: Arc<BTreeMap<String, Upstream>>,
}

impl std::fmt::Debug for UpstreamRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpstreamRegistry")
            .field("count", &self.inner.len())
            .field("names", &self.inner.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl UpstreamRegistry {
    /// Build a registry from the upstream map parsed out of `routes.yaml`.
    ///
    /// When a single upstream declares multiple endpoints, tonic's
    /// [`Channel::balance_list`] distributes calls across them round-robin.
    pub fn from_config(upstreams: &BTreeMap<String, UpstreamConfig>) -> AppResult<Self> {
        let mut inner = BTreeMap::new();

        for (name, cfg) in upstreams {
            let mut endpoints = Vec::with_capacity(cfg.endpoints.len());
            for ep in &cfg.endpoints {
                let endpoint = Endpoint::from_shared(ep.clone()).map_err(|e| {
                    AppError::Internal {
                        message: format!(
                            "upstream `{name}`: invalid endpoint `{ep}`: {e}"
                        ),
                    }
                })?;
                endpoints.push(endpoint);
            }

            let channel = if endpoints.len() == 1 {
                endpoints.into_iter().next().unwrap().connect_lazy()
            } else {
                Channel::balance_list(endpoints.into_iter())
            };

            inner.insert(
                name.clone(),
                Upstream {
                    name: name.clone(),
                    channel,
                    timeout: Duration::from_millis(cfg.timeout_ms),
                },
            );
        }

        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    /// Return the upstream with the given name.
    pub fn get(&self, name: &str) -> Option<&Upstream> {
        self.inner.get(name)
    }

    /// Number of registered upstreams.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether there are no registered upstreams.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge::route_config::RouteConfigFile;

    const CFG: &str = r#"
upstreams:
  order-svc:
    endpoints: ["http://127.0.0.1:50051"]
    timeout_ms: 4000
  multi:
    endpoints: ["http://127.0.0.1:50051", "http://127.0.0.1:50052"]
routes: []
"#;

    // Requires a tokio runtime because `Channel::balance_list` spawns a task
    // internally.
    #[tokio::test]
    async fn builds_registry_from_yaml() {
        let cfg = RouteConfigFile::from_yaml(CFG).unwrap();
        let reg = UpstreamRegistry::from_config(&cfg.upstreams).unwrap();
        assert_eq!(reg.len(), 2);
        let up = reg.get("order-svc").expect("order-svc present");
        assert_eq!(up.timeout, Duration::from_millis(4000));
        assert!(reg.get("unknown").is_none());
    }

    #[test]
    fn rejects_invalid_endpoint() {
        let cfg = RouteConfigFile::from_yaml(
            r#"
upstreams:
  bad:
    endpoints: ["not a uri"]
routes: []
"#,
        )
        .unwrap();
        let err = UpstreamRegistry::from_config(&cfg.upstreams).unwrap_err();
        assert!(format!("{err}").contains("invalid endpoint"));
    }
}
