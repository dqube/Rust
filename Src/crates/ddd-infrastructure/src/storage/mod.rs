//! Object-storage adapters.
//!
//! Enabled with the `storage` feature.  Currently ships one adapter —
//! [`S3BlobStorage`] — which speaks the S3 REST API and therefore covers
//! AWS S3, MinIO, and SeaweedFS from a single implementation.

pub mod s3;

pub use s3::{S3BlobStorage, S3Config};
