//! Presigned-URL object key helpers.
//!
//! The real blob storage port lives in `ddd_shared_kernel::BlobStorage`
//! and adapters are in `ddd_infrastructure::S3BlobStorage`. This module
//! only defines service-local conventions around object key layouts used
//! by the customer aggregate.

use crate::domain::ids::CustomerId;
use uuid::Uuid;

/// Build an `avatars/{customer_id}/{uuid}-{sanitized_filename}` key.
pub fn avatar_object_key(customer_id: CustomerId, filename: &str) -> String {
    format!(
        "avatars/{customer_id}/{}-{}",
        Uuid::now_v7().simple(),
        sanitize_filename(filename)
    )
}

/// Build a `kyc/{customer_id}/{document_type}/{uuid}-{sanitized_filename}` key.
pub fn kyc_object_key(customer_id: CustomerId, document_type: &str, filename: &str) -> String {
    let doc_type = if document_type.trim().is_empty() {
        "unknown".to_owned()
    } else {
        document_type.replace(['/', '\\'], "_")
    };
    format!(
        "kyc/{customer_id}/{doc_type}/{}-{}",
        Uuid::now_v7().simple(),
        sanitize_filename(filename)
    )
}

fn sanitize_filename(filename: &str) -> String {
    let trimmed = filename.trim();
    if trimmed.is_empty() {
        return "file.bin".to_owned();
    }
    trimmed
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("file.bin")
        .to_owned()
}
