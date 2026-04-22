//! Edge helpers shared by every BFF binary.
//!
//! Currently only provides graceful-shutdown signal handling; the
//! previous config-driven router / hyper server / upstream registry
//! (intended for a Pingora-style YAML-routed edge) has been removed —
//! services wire their own axum routers directly.

pub mod shutdown;

pub use shutdown::{drain_with_timeout, install_signal_handler, wait_for_shutdown_signal};
