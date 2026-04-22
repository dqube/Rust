//! Cache adapters for the [`Cache`] port.
//!
//! Enabled with the `cache` feature.  Currently ships [`RedisCache`].

pub mod redis;

pub use redis::RedisCache;
