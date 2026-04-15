//! Process-wide [`prost_reflect::DescriptorPool`].
//!
//! Consumers install their own pool once at startup via [`install`].
//! The library itself only needs the pool when transcoding REST → gRPC.
//!
//! ## Install
//!
//! ```no_run
//! # use ddd_bff::transcode;
//! # fn load_my_descriptor_set() -> Vec<u8> { Vec::new() }
//! let bytes = load_my_descriptor_set(); // produced by tonic-build / prost-build
//! let pool = transcode::decode_pool(&bytes).expect("descriptor pool");
//! transcode::install(pool).expect("install once");
//! ```
//!
//! For the library's own tests an embedded fixture descriptor is bundled
//! and used automatically when no pool has been installed.

use std::sync::OnceLock;

use ddd_shared_kernel::{AppError, AppResult};
use prost_reflect::DescriptorPool;

/// Bundled fixture descriptor used by the library tests when no pool has
/// been installed. Compiled from `proto/fixture.proto`. Consumers should
/// call [`install`] with their own pool.
const FIXTURE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/descriptor.bin"));

static POOL: OnceLock<DescriptorPool> = OnceLock::new();

/// Install the process-wide descriptor pool.
///
/// Idempotent: returns an error if a pool has already been installed.
pub fn install(pool: DescriptorPool) -> AppResult<()> {
    POOL.set(pool)
        .map_err(|_| AppError::internal("descriptor pool already installed"))
}

/// Decode a serialised `FileDescriptorSet` into a [`DescriptorPool`].
///
/// The `bytes` argument is typically the contents of a `descriptor.bin`
/// produced by `tonic-build` / `prost-build`'s
/// `file_descriptor_set_path()`.
pub fn decode_pool(bytes: &[u8]) -> AppResult<DescriptorPool> {
    DescriptorPool::decode(bytes).map_err(|e| AppError::Internal {
        message: format!("failed to decode proto descriptor pool: {e}"),
    })
}

/// Return the process-wide descriptor pool.
///
/// If no pool has been installed via [`install`], the bundled fixture
/// descriptor is decoded and returned (cached). This keeps the library's
/// own tests self-contained while still allowing consumers to override.
pub fn load() -> AppResult<&'static DescriptorPool> {
    if let Some(pool) = POOL.get() {
        return Ok(pool);
    }
    let pool = decode_pool(FIXTURE_BYTES)?;
    Ok(POOL.get_or_init(|| pool))
}

/// Raw bundled fixture bytes — exposed for tests / diagnostics only.
#[doc(hidden)]
pub fn pool_bytes() -> &'static [u8] {
    FIXTURE_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_bytes_non_empty() {
        assert!(!pool_bytes().is_empty(), "fixture descriptor must not be empty");
    }

    #[test]
    fn pool_loads_and_caches() {
        let first = load().expect("descriptor pool must load");
        let second = load().expect("second load should return cached pool");
        assert!(
            std::ptr::eq(first, second),
            "load() must return the same cached pool reference"
        );
    }

    #[test]
    fn double_install_fails() {
        // First install (or fixture fallback) wins. A second install must error.
        let _ = load().expect("load fixture");
        let bytes = pool_bytes().to_vec();
        let pool = decode_pool(&bytes).expect("decode again");
        assert!(install(pool).is_err());
    }
}
