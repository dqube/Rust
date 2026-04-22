//! Customer domain + integration events.

use chrono::{DateTime, Utc};
use ddd_shared_kernel::{DomainEvent, IntegrationEvent};
use serde::{Deserialize, Serialize};
use std::any::Any;
use uuid::Uuid;

use super::ids::CustomerId;

macro_rules! domain_event {
    ($ty:ident, $name:literal) => {
        impl DomainEvent for $ty {
            fn event_name(&self) -> &'static str {
                $name
            }
            fn occurred_at(&self) -> DateTime<Utc> {
                self.occurred_at
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}

macro_rules! integration_event {
    ($ty:ident, $type:literal, $subject:literal) => {
        impl IntegrationEvent for $ty {
            fn event_type(&self) -> &'static str {
                $type
            }
            fn subject(&self) -> String {
                $subject.to_owned()
            }
            fn occurred_at(&self) -> DateTime<Utc> {
                self.occurred_at
            }
        }
    };
}

// ── Customer lifecycle ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerCreated {
    pub customer_id: CustomerId,
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub membership_number: String,
    pub join_date: DateTime<Utc>,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(CustomerCreated, "customer.customer.created");
integration_event!(
    CustomerCreated,
    "customer.customer.created.v1",
    "customer.customer.created"
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerEmailUpdated {
    pub customer_id: CustomerId,
    pub old_email: Option<String>,
    pub new_email: Option<String>,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(CustomerEmailUpdated, "customer.customer.email_updated");
integration_event!(
    CustomerEmailUpdated,
    "customer.customer.email_updated.v1",
    "customer.customer.email_updated"
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoyaltyPointsUpdated {
    pub customer_id: CustomerId,
    pub previous_points: i32,
    pub new_points: i32,
    pub reason: String,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(LoyaltyPointsUpdated, "customer.customer.loyalty_updated");
integration_event!(
    LoyaltyPointsUpdated,
    "customer.customer.loyalty_updated.v1",
    "customer.customer.loyalty_updated"
);

// ── KYC ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycVerified {
    pub customer_id: CustomerId,
    pub verified_at: DateTime<Utc>,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(KycVerified, "customer.customer.kyc_verified");
integration_event!(
    KycVerified,
    "customer.customer.kyc_verified.v1",
    "customer.customer.kyc_verified"
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycRejected {
    pub customer_id: CustomerId,
    pub rejection_reason: String,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(KycRejected, "customer.customer.kyc_rejected");
integration_event!(
    KycRejected,
    "customer.customer.kyc_rejected.v1",
    "customer.customer.kyc_rejected"
);

// ── Wishlist ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WishlistItemAdded {
    pub customer_id: CustomerId,
    pub product_id: Uuid,
    pub product_name: String,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(WishlistItemAdded, "customer.wishlist.item_added");
integration_event!(
    WishlistItemAdded,
    "customer.wishlist.item_added.v1",
    "customer.wishlist.item_added"
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WishlistItemRemoved {
    pub customer_id: CustomerId,
    pub product_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(WishlistItemRemoved, "customer.wishlist.item_removed");
integration_event!(
    WishlistItemRemoved,
    "customer.wishlist.item_removed.v1",
    "customer.wishlist.item_removed"
);
