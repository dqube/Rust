//! Repository ports for the customer aggregates.
//!
//! Pagination follows the workspace-standard [`Page`] / [`PageRequest`] shape
//! (same as product-service + auth-service). Implementations live in
//! `infrastructure::db`.

use async_trait::async_trait;
use ddd_shared_kernel::{AppResult, Page, PageRequest};
use uuid::Uuid;

use super::entities::{Customer, CustomerProfile, WishlistItem};
use super::enums::KycStatus;
use super::ids::{CustomerId, CustomerProfileId, WishlistItemId};

/// Filters applied to `CustomerRepository::list_paged`.
#[derive(Debug, Default, Clone)]
pub struct CustomerListFilter {
    /// Case-insensitive substring match against first/last name or email.
    pub search: Option<String>,
    /// Filter by ISO country code (exact match).
    pub country_code: Option<String>,
    pub is_membership_active: Option<bool>,
}

#[async_trait]
pub trait CustomerRepository: Send + Sync {
    async fn find_by_id(&self, id: CustomerId) -> AppResult<Option<Customer>>;
    async fn find_with_children(&self, id: CustomerId) -> AppResult<Option<Customer>>;
    async fn find_by_user_id(&self, user_id: Uuid) -> AppResult<Option<Customer>>;
    async fn find_by_email(&self, email: &str) -> AppResult<Option<Customer>>;
    async fn email_exists(&self, email: &str) -> AppResult<bool>;
    async fn membership_number_exists(&self, number: &str) -> AppResult<bool>;
    async fn list_paged(
        &self,
        page: PageRequest,
        filter: CustomerListFilter,
    ) -> AppResult<Page<Customer>>;
    async fn save(&self, customer: &Customer) -> AppResult<()>;
}

#[async_trait]
pub trait CustomerProfileRepository: Send + Sync {
    async fn find_by_id(&self, id: CustomerProfileId) -> AppResult<Option<CustomerProfile>>;
    async fn find_by_customer_id(
        &self,
        customer_id: CustomerId,
    ) -> AppResult<Option<CustomerProfile>>;
    async fn exists_by_customer_id(&self, customer_id: CustomerId) -> AppResult<bool>;
    async fn list_by_kyc_status(
        &self,
        status: KycStatus,
        page: PageRequest,
    ) -> AppResult<Page<CustomerProfile>>;
    async fn save(&self, profile: &CustomerProfile) -> AppResult<()>;
}

#[async_trait]
pub trait WishlistItemRepository: Send + Sync {
    async fn find_by_id(&self, id: WishlistItemId) -> AppResult<Option<WishlistItem>>;
    async fn find_by_customer_and_product(
        &self,
        customer_id: CustomerId,
        product_id: Uuid,
    ) -> AppResult<Option<WishlistItem>>;
    async fn list_by_customer_id(&self, customer_id: CustomerId) -> AppResult<Vec<WishlistItem>>;
    async fn save(&self, item: &WishlistItem) -> AppResult<()>;
    async fn delete(&self, id: WishlistItemId) -> AppResult<()>;
    async fn delete_all_by_customer_id(&self, customer_id: CustomerId) -> AppResult<()>;
}
