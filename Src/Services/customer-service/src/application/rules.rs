//! Business rules used by the command handlers.
//!
//! Each rule returns `AppResult<()>` — `Ok(())` when the rule holds, and an
//! [`AppError`] (`Conflict`, `NotFound`, or `Validation`) when it is
//! violated. Callers bubble these errors up so the gRPC adapter can map them
//! onto the right `tonic::Status`.

use std::sync::Arc;

use ddd_shared_kernel::{AppError, AppResult};
use uuid::Uuid;

use crate::domain::ids::CustomerId;
use crate::domain::repositories::{
    CustomerProfileRepository, CustomerRepository, WishlistItemRepository,
};

// ── Uniqueness ──────────────────────────────────────────────────────────────

/// Email must not already be in use. Checked on `CreateCustomer`.
pub async fn customer_email_must_be_unique(
    email: Option<&str>,
    repo: &Arc<dyn CustomerRepository>,
) -> AppResult<()> {
    if let Some(e) = email.map(str::trim).filter(|s| !s.is_empty()) {
        if repo.email_exists(e).await? {
            return Err(AppError::conflict(format!(
                "email {e} is already registered"
            )));
        }
    }
    Ok(())
}

/// Membership number must be unique across all customers. Checked on create.
pub async fn membership_number_must_be_unique(
    number: &str,
    repo: &Arc<dyn CustomerRepository>,
) -> AppResult<()> {
    let trimmed = number.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    if repo.membership_number_exists(trimmed).await? {
        return Err(AppError::conflict(format!(
            "membership number {trimmed} is already in use"
        )));
    }
    Ok(())
}

// ── Existence ──────────────────────────────────────────────────────────────

/// A customer with the given id must exist.
pub async fn customer_must_exist(
    customer_id: CustomerId,
    repo: &Arc<dyn CustomerRepository>,
) -> AppResult<()> {
    if repo.find_by_id(customer_id).await?.is_none() {
        return Err(AppError::not_found("Customer", customer_id.to_string()));
    }
    Ok(())
}

/// A customer profile for the given customer must not already exist.
pub async fn customer_profile_must_not_exist(
    customer_id: CustomerId,
    repo: &Arc<dyn CustomerProfileRepository>,
) -> AppResult<()> {
    if repo.exists_by_customer_id(customer_id).await? {
        return Err(AppError::conflict(
            "customer profile already exists for this customer",
        ));
    }
    Ok(())
}

/// A wishlist item for the given product must not already exist for the
/// customer. Keeps the gRPC contract clean (409 conflict) instead of silently
/// idempotent.
pub async fn wishlist_item_must_not_exist(
    customer_id: CustomerId,
    product_id: Uuid,
    repo: &Arc<dyn WishlistItemRepository>,
) -> AppResult<()> {
    if repo
        .find_by_customer_and_product(customer_id, product_id)
        .await?
        .is_some()
    {
        return Err(AppError::conflict(
            "product is already on the customer's wishlist",
        ));
    }
    Ok(())
}
