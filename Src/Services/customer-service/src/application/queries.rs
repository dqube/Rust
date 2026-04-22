use chrono::{DateTime, Utc};
use ddd_shared_kernel::Page;
use uuid::Uuid;

use crate::domain::entities::{Customer, CustomerProfile, WishlistItem};
use crate::domain::ids::CustomerId;

// ── Customer ───────────────────────────────────────────────────────────────

pub struct GetCustomerById {
    pub customer_id: CustomerId,
}
ddd_application::impl_query!(GetCustomerById, Option<Customer>);

pub struct GetCustomerByUserId {
    pub user_id: Uuid,
}
ddd_application::impl_query!(GetCustomerByUserId, Option<Customer>);

pub struct ListCustomers {
    pub page: u32,
    pub per_page: u32,
    pub search: Option<String>,
    pub country_code: Option<String>,
}
ddd_application::impl_query!(ListCustomers, Page<Customer>);

// ── Profile ────────────────────────────────────────────────────────────────

pub struct GetCustomerProfile {
    pub customer_id: CustomerId,
}
ddd_application::impl_query!(GetCustomerProfile, Option<CustomerProfile>);

// ── Avatar ─────────────────────────────────────────────────────────────────

pub struct GetCustomerAvatarUrl {
    pub customer_id: CustomerId,
}

pub struct AvatarLink {
    pub has_avatar: bool,
    pub avatar_url: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}
ddd_application::impl_query!(GetCustomerAvatarUrl, AvatarLink);

// ── Wishlist ───────────────────────────────────────────────────────────────

pub struct GetWishlist {
    pub customer_id: CustomerId,
}
ddd_application::impl_query!(GetWishlist, Vec<WishlistItem>);
