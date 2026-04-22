//! Write-side command definitions. Handlers live in `handlers.rs`.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::enums::Gender;
use crate::domain::ids::{CustomerAddressId, CustomerId, CustomerProfileId};

// ── Customer CRUD ──────────────────────────────────────────────────────────

pub struct CreateCustomer {
    pub user_id: Option<Uuid>,
    pub first_name: String,
    pub last_name: String,
    pub country_code: String,
    pub email: Option<String>,
    pub membership_number: Option<String>,
    pub join_date: Option<DateTime<Utc>>,
    pub expiry_date: Option<DateTime<Utc>>,
}

/// Returns `(customer_id, membership_number)`.
pub struct CreateCustomerResult {
    pub customer_id: CustomerId,
    pub membership_number: String,
}
ddd_application::impl_command!(CreateCustomer, CreateCustomerResult);

/// Idempotent: creates a customer for the given `user_id` if none exists.
pub struct EnsureCustomerExists {
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
}
ddd_application::impl_command!(EnsureCustomerExists, CustomerId);

pub struct UpdateCustomerInfo {
    pub customer_id: CustomerId,
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
}
ddd_application::impl_command!(UpdateCustomerInfo, ());

// ── Loyalty ────────────────────────────────────────────────────────────────

pub struct AddLoyaltyPoints {
    pub customer_id: CustomerId,
    pub points: i32,
    pub reason: String,
}
ddd_application::impl_command!(AddLoyaltyPoints, i32);

pub struct RedeemLoyaltyPoints {
    pub customer_id: CustomerId,
    pub points: i32,
    pub reason: String,
}
ddd_application::impl_command!(RedeemLoyaltyPoints, i32);

// ── Addresses ──────────────────────────────────────────────────────────────

pub struct AddCustomerAddress {
    pub customer_id: CustomerId,
    pub label: String,
    pub line1: String,
    pub line2: Option<String>,
    pub city: String,
    pub state: Option<String>,
    pub postal_code: String,
    pub country_code: String,
    pub is_default: bool,
}
ddd_application::impl_command!(AddCustomerAddress, CustomerAddressId);

pub struct UpdateCustomerAddress {
    pub customer_id: CustomerId,
    pub address_id: CustomerAddressId,
    pub label: String,
    pub line1: String,
    pub line2: Option<String>,
    pub city: String,
    pub state: Option<String>,
    pub postal_code: String,
    pub country_code: String,
}
ddd_application::impl_command!(UpdateCustomerAddress, ());

pub struct RemoveCustomerAddress {
    pub customer_id: CustomerId,
    pub address_id: CustomerAddressId,
}
ddd_application::impl_command!(RemoveCustomerAddress, ());

pub struct SetDefaultCustomerAddress {
    pub customer_id: CustomerId,
    pub address_id: CustomerAddressId,
}
ddd_application::impl_command!(SetDefaultCustomerAddress, ());

// ── Avatar (presigned workflow) ────────────────────────────────────────────

pub struct RequestAvatarUploadUrl {
    pub customer_id: CustomerId,
    pub filename: String,
    pub content_type: String,
}

pub struct UploadUrl {
    pub upload_url: String,
    pub object_key: String,
    pub expires_in_secs: i64,
}
ddd_application::impl_command!(RequestAvatarUploadUrl, UploadUrl);

pub struct ConfirmAvatarUpload {
    pub customer_id: CustomerId,
    pub object_key: String,
}
ddd_application::impl_command!(ConfirmAvatarUpload, ());

// ── Profile ────────────────────────────────────────────────────────────────

pub struct CreateCustomerProfile {
    pub customer_id: CustomerId,
    pub preferred_language: String,
    pub preferred_currency: String,
    pub date_of_birth: Option<DateTime<Utc>>,
    pub gender: Option<Gender>,
    pub tax_id: Option<String>,
    pub company_registration_number: Option<String>,
    pub email_notifications: bool,
    pub sms_notifications: bool,
    pub push_notifications: bool,
    pub marketing_emails: bool,
}
ddd_application::impl_command!(CreateCustomerProfile, CustomerProfileId);

pub struct UpdateCustomerProfile {
    pub customer_id: CustomerId,
    pub date_of_birth: Option<DateTime<Utc>>,
    pub set_date_of_birth: bool,
    pub gender: Option<Gender>,
    pub set_gender: bool,
    pub preferred_language: Option<String>,
    pub preferred_currency: Option<String>,
    pub tax_id: Option<String>,
    pub set_tax_id: bool,
    pub company_registration_number: Option<String>,
    pub set_company_registration_number: bool,
}
ddd_application::impl_command!(UpdateCustomerProfile, ());

pub struct UpdateNotificationPreferences {
    pub customer_id: CustomerId,
    pub email_notifications: bool,
    pub sms_notifications: bool,
    pub push_notifications: bool,
    pub marketing_emails: bool,
}
ddd_application::impl_command!(UpdateNotificationPreferences, ());

// ── KYC ────────────────────────────────────────────────────────────────────

pub struct SubmitKycDocument {
    pub customer_id: CustomerId,
    pub document_type: String,
    pub document_number: String,
    pub file_url: String,
}

pub struct KycSubmissionResult {
    pub kyc_status: String,
    pub document_count: i32,
}
ddd_application::impl_command!(SubmitKycDocument, KycSubmissionResult);

pub struct RequestKycDocumentUploadUrl {
    pub customer_id: CustomerId,
    pub document_type: String,
    pub filename: String,
    pub content_type: String,
}
ddd_application::impl_command!(RequestKycDocumentUploadUrl, UploadUrl);

pub struct SubmitForKycReview {
    pub customer_id: CustomerId,
}
ddd_application::impl_command!(SubmitForKycReview, ());

pub struct VerifyKyc {
    pub customer_id: CustomerId,
    pub verified_by: Uuid,
}

pub struct VerifyKycResult {
    pub kyc_status: String,
    pub kyc_verified_at: DateTime<Utc>,
}
ddd_application::impl_command!(VerifyKyc, VerifyKycResult);

pub struct RejectKyc {
    pub customer_id: CustomerId,
    pub rejection_reason: String,
    pub rejected_by: Uuid,
}
ddd_application::impl_command!(RejectKyc, String);

// ── Wishlist ───────────────────────────────────────────────────────────────

pub struct AddToWishlist {
    pub customer_id: CustomerId,
    pub product_id: Uuid,
    pub product_name: String,
    pub base_price: f64,
}

pub struct WishlistItemAddedResult {
    pub id: String,
    pub product_id: Uuid,
    pub added_at: DateTime<Utc>,
}
ddd_application::impl_command!(AddToWishlist, WishlistItemAddedResult);

pub struct RemoveFromWishlist {
    pub customer_id: CustomerId,
    pub product_id: Uuid,
}
ddd_application::impl_command!(RemoveFromWishlist, bool);

pub struct ClearWishlist {
    pub customer_id: CustomerId,
}
ddd_application::impl_command!(ClearWishlist, ());
