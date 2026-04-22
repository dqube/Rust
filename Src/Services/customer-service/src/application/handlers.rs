//! Command + query handlers wired via the mediator.
//!
//! Handlers are intentionally thin — business logic lives on the aggregates
//! in `crate::domain::entities`; uniqueness + existence rules live in
//! `crate::application::rules`.

use std::sync::Arc;
use std::time::Duration as StdDuration;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use ddd_application::{
    register_command_handler, register_query_handler, CommandHandler, QueryHandler,
};
use ddd_shared_kernel::{AppError, AppResult, BlobStorage, Page, PageRequest};
use uuid::Uuid;

use super::commands::{
    AddCustomerAddress, AddLoyaltyPoints, AddToWishlist, ClearWishlist, ConfirmAvatarUpload,
    CreateCustomer, CreateCustomerProfile, CreateCustomerResult, EnsureCustomerExists,
    KycSubmissionResult, RedeemLoyaltyPoints, RejectKyc, RemoveCustomerAddress,
    RemoveFromWishlist, RequestAvatarUploadUrl, RequestKycDocumentUploadUrl,
    SetDefaultCustomerAddress, SubmitForKycReview, SubmitKycDocument, UpdateCustomerAddress,
    UpdateCustomerInfo, UpdateCustomerProfile, UpdateNotificationPreferences, UploadUrl, VerifyKyc,
    VerifyKycResult, WishlistItemAddedResult,
};
use super::deps::AppDeps;
use super::queries::{
    AvatarLink, GetCustomerAvatarUrl, GetCustomerById, GetCustomerByUserId, GetCustomerProfile,
    GetWishlist, ListCustomers,
};
use super::rules::{
    customer_email_must_be_unique, customer_must_exist, customer_profile_must_not_exist,
    membership_number_must_be_unique, wishlist_item_must_not_exist,
};
use crate::domain::blob::{avatar_object_key, kyc_object_key};
use crate::domain::entities::{
    Customer, CustomerAddress, CustomerProfile, KycDocument, WishlistItem,
};
use crate::domain::enums::AddressType;
use crate::domain::ids::{CustomerAddressId, CustomerId, CustomerProfileId};
use crate::domain::repositories::{
    CustomerListFilter, CustomerProfileRepository, CustomerRepository, WishlistItemRepository,
};

// ── Helpers ────────────────────────────────────────────────────────────────

fn default_membership_number() -> String {
    let ts = Utc::now().format("%Y%m%d");
    let tail = Uuid::now_v7().simple().to_string();
    let short: String = tail.chars().take(8).collect();
    format!("MEM{ts}{short}").to_uppercase()
}

async fn get_or_create_profile(
    customer_id: CustomerId,
    profile_repo: &Arc<dyn CustomerProfileRepository>,
) -> AppResult<CustomerProfile> {
    if let Some(existing) = profile_repo.find_by_customer_id(customer_id).await? {
        return Ok(existing);
    }
    CustomerProfile::new(
        CustomerProfileId::new(),
        customer_id,
        "en".to_owned(),
        "USD".to_owned(),
    )
}

// ────────────────────────────────────────────────────────────────────────────
// Customer CRUD
// ────────────────────────────────────────────────────────────────────────────

pub struct CreateCustomerHandler {
    repo: Arc<dyn CustomerRepository>,
}

#[async_trait]
impl CommandHandler<CreateCustomer> for CreateCustomerHandler {
    async fn handle(&self, cmd: CreateCustomer) -> AppResult<CreateCustomerResult> {
        customer_email_must_be_unique(cmd.email.as_deref(), &self.repo).await?;

        let membership_number = cmd
            .membership_number
            .map(|m| m.trim().to_owned())
            .filter(|m| !m.is_empty())
            .unwrap_or_else(default_membership_number);
        membership_number_must_be_unique(&membership_number, &self.repo).await?;

        let join = cmd.join_date.unwrap_or_else(Utc::now);
        let expiry = cmd
            .expiry_date
            .unwrap_or_else(|| join + Duration::days(365));

        let id = CustomerId::new();
        let mut customer = Customer::create(
            id,
            cmd.user_id.unwrap_or_else(Uuid::now_v7),
            cmd.first_name,
            cmd.last_name,
            cmd.email,
            membership_number.clone(),
            join,
            expiry,
            cmd.country_code,
        )?;

        self.repo.save(&customer).await?;
        let _ = customer.drain_events();

        Ok(CreateCustomerResult {
            customer_id: id,
            membership_number,
        })
    }
}

register_command_handler!(CreateCustomer, AppDeps, |deps: &AppDeps| {
    CreateCustomerHandler {
        repo: deps.customer_repo.clone(),
    }
});

pub struct EnsureCustomerExistsHandler {
    repo: Arc<dyn CustomerRepository>,
}

#[async_trait]
impl CommandHandler<EnsureCustomerExists> for EnsureCustomerExistsHandler {
    async fn handle(&self, cmd: EnsureCustomerExists) -> AppResult<CustomerId> {
        if let Some(existing) = self.repo.find_by_user_id(cmd.user_id).await? {
            return Ok(existing.id);
        }
        let membership_number = default_membership_number();
        let id = CustomerId::new();
        let now = Utc::now();
        let mut customer = Customer::create(
            id,
            cmd.user_id,
            cmd.first_name,
            cmd.last_name,
            cmd.email,
            membership_number,
            now,
            now + Duration::days(365),
            "US".to_owned(),
        )?;
        self.repo.save(&customer).await?;
        let _ = customer.drain_events();
        Ok(id)
    }
}

register_command_handler!(EnsureCustomerExists, AppDeps, |deps: &AppDeps| {
    EnsureCustomerExistsHandler {
        repo: deps.customer_repo.clone(),
    }
});

pub struct UpdateCustomerInfoHandler {
    repo: Arc<dyn CustomerRepository>,
}

#[async_trait]
impl CommandHandler<UpdateCustomerInfo> for UpdateCustomerInfoHandler {
    async fn handle(&self, cmd: UpdateCustomerInfo) -> AppResult<()> {
        let mut customer = self
            .repo
            .find_with_children(cmd.customer_id)
            .await?
            .ok_or_else(|| AppError::not_found("Customer", cmd.customer_id.to_string()))?;
        customer.update_personal_info(cmd.first_name, cmd.last_name)?;
        if let Some(phone) = cmd.phone.filter(|p| !p.trim().is_empty()) {
            customer.set_primary_contact_number(
                crate::domain::enums::ContactNumberType::Mobile,
                phone,
            );
        }
        self.repo.save(&customer).await
    }
}

register_command_handler!(UpdateCustomerInfo, AppDeps, |deps: &AppDeps| {
    UpdateCustomerInfoHandler {
        repo: deps.customer_repo.clone(),
    }
});

// ────────────────────────────────────────────────────────────────────────────
// Loyalty
// ────────────────────────────────────────────────────────────────────────────

pub struct AddLoyaltyPointsHandler {
    repo: Arc<dyn CustomerRepository>,
}

#[async_trait]
impl CommandHandler<AddLoyaltyPoints> for AddLoyaltyPointsHandler {
    async fn handle(&self, cmd: AddLoyaltyPoints) -> AppResult<i32> {
        let mut customer = self
            .repo
            .find_with_children(cmd.customer_id)
            .await?
            .ok_or_else(|| AppError::not_found("Customer", cmd.customer_id.to_string()))?;
        customer.add_loyalty_points(cmd.points, cmd.reason)?;
        let total = customer.loyalty_points;
        self.repo.save(&customer).await?;
        let _ = customer.drain_events();
        Ok(total)
    }
}

register_command_handler!(AddLoyaltyPoints, AppDeps, |deps: &AppDeps| {
    AddLoyaltyPointsHandler {
        repo: deps.customer_repo.clone(),
    }
});

pub struct RedeemLoyaltyPointsHandler {
    repo: Arc<dyn CustomerRepository>,
}

#[async_trait]
impl CommandHandler<RedeemLoyaltyPoints> for RedeemLoyaltyPointsHandler {
    async fn handle(&self, cmd: RedeemLoyaltyPoints) -> AppResult<i32> {
        let mut customer = self
            .repo
            .find_with_children(cmd.customer_id)
            .await?
            .ok_or_else(|| AppError::not_found("Customer", cmd.customer_id.to_string()))?;
        customer.redeem_loyalty_points(cmd.points, cmd.reason)?;
        let total = customer.loyalty_points;
        self.repo.save(&customer).await?;
        let _ = customer.drain_events();
        Ok(total)
    }
}

register_command_handler!(RedeemLoyaltyPoints, AppDeps, |deps: &AppDeps| {
    RedeemLoyaltyPointsHandler {
        repo: deps.customer_repo.clone(),
    }
});

// ────────────────────────────────────────────────────────────────────────────
// Addresses
// ────────────────────────────────────────────────────────────────────────────

pub struct AddCustomerAddressHandler {
    repo: Arc<dyn CustomerRepository>,
}

#[async_trait]
impl CommandHandler<AddCustomerAddress> for AddCustomerAddressHandler {
    async fn handle(&self, cmd: AddCustomerAddress) -> AppResult<CustomerAddressId> {
        let mut customer = self
            .repo
            .find_with_children(cmd.customer_id)
            .await?
            .ok_or_else(|| AppError::not_found("Customer", cmd.customer_id.to_string()))?;
        let id = CustomerAddressId::new();
        let addr = CustomerAddress::new(
            id,
            cmd.customer_id,
            cmd.label,
            AddressType::Home,
            cmd.line1,
            cmd.line2,
            cmd.city,
            cmd.state,
            cmd.postal_code,
            cmd.country_code,
            cmd.is_default,
        )?;
        customer.add_address(addr);
        self.repo.save(&customer).await?;
        Ok(id)
    }
}

register_command_handler!(AddCustomerAddress, AppDeps, |deps: &AppDeps| {
    AddCustomerAddressHandler {
        repo: deps.customer_repo.clone(),
    }
});

pub struct UpdateCustomerAddressHandler {
    repo: Arc<dyn CustomerRepository>,
}

#[async_trait]
impl CommandHandler<UpdateCustomerAddress> for UpdateCustomerAddressHandler {
    async fn handle(&self, cmd: UpdateCustomerAddress) -> AppResult<()> {
        let mut customer = self
            .repo
            .find_with_children(cmd.customer_id)
            .await?
            .ok_or_else(|| AppError::not_found("Customer", cmd.customer_id.to_string()))?;
        customer.update_address(
            cmd.address_id,
            cmd.label,
            cmd.line1,
            cmd.line2,
            cmd.city,
            cmd.state,
            cmd.postal_code,
            cmd.country_code,
        )?;
        self.repo.save(&customer).await
    }
}

register_command_handler!(UpdateCustomerAddress, AppDeps, |deps: &AppDeps| {
    UpdateCustomerAddressHandler {
        repo: deps.customer_repo.clone(),
    }
});

pub struct RemoveCustomerAddressHandler {
    repo: Arc<dyn CustomerRepository>,
}

#[async_trait]
impl CommandHandler<RemoveCustomerAddress> for RemoveCustomerAddressHandler {
    async fn handle(&self, cmd: RemoveCustomerAddress) -> AppResult<()> {
        let mut customer = self
            .repo
            .find_with_children(cmd.customer_id)
            .await?
            .ok_or_else(|| AppError::not_found("Customer", cmd.customer_id.to_string()))?;
        customer.remove_address(cmd.address_id)?;
        self.repo.save(&customer).await
    }
}

register_command_handler!(RemoveCustomerAddress, AppDeps, |deps: &AppDeps| {
    RemoveCustomerAddressHandler {
        repo: deps.customer_repo.clone(),
    }
});

pub struct SetDefaultCustomerAddressHandler {
    repo: Arc<dyn CustomerRepository>,
}

#[async_trait]
impl CommandHandler<SetDefaultCustomerAddress> for SetDefaultCustomerAddressHandler {
    async fn handle(&self, cmd: SetDefaultCustomerAddress) -> AppResult<()> {
        let mut customer = self
            .repo
            .find_with_children(cmd.customer_id)
            .await?
            .ok_or_else(|| AppError::not_found("Customer", cmd.customer_id.to_string()))?;
        customer.set_default_address(cmd.address_id)?;
        self.repo.save(&customer).await
    }
}

register_command_handler!(SetDefaultCustomerAddress, AppDeps, |deps: &AppDeps| {
    SetDefaultCustomerAddressHandler {
        repo: deps.customer_repo.clone(),
    }
});

// ────────────────────────────────────────────────────────────────────────────
// Avatar
// ────────────────────────────────────────────────────────────────────────────

pub struct RequestAvatarUploadUrlHandler {
    customer_repo: Arc<dyn CustomerRepository>,
    blob: Arc<dyn BlobStorage>,
    bucket: String,
    ttl_secs: u64,
}

#[async_trait]
impl CommandHandler<RequestAvatarUploadUrl> for RequestAvatarUploadUrlHandler {
    async fn handle(&self, cmd: RequestAvatarUploadUrl) -> AppResult<UploadUrl> {
        customer_must_exist(cmd.customer_id, &self.customer_repo).await?;
        let object_key = avatar_object_key(cmd.customer_id, &cmd.filename);
        let content_type = if cmd.content_type.trim().is_empty() {
            "application/octet-stream".to_owned()
        } else {
            cmd.content_type
        };
        let ttl = StdDuration::from_secs(self.ttl_secs);
        let link = self
            .blob
            .presigned_put(&self.bucket, &object_key, &content_type, ttl)
            .await?;
        let expires_in_secs = (link.expires_at - Utc::now()).num_seconds().max(0);
        Ok(UploadUrl {
            upload_url: link.url,
            object_key,
            expires_in_secs,
        })
    }
}

register_command_handler!(RequestAvatarUploadUrl, AppDeps, |deps: &AppDeps| {
    RequestAvatarUploadUrlHandler {
        customer_repo: deps.customer_repo.clone(),
        blob: deps.blob_storage.clone(),
        bucket: deps.blob_bucket.clone(),
        ttl_secs: deps.presign_ttl_secs,
    }
});

pub struct ConfirmAvatarUploadHandler {
    profile_repo: Arc<dyn CustomerProfileRepository>,
}

#[async_trait]
impl CommandHandler<ConfirmAvatarUpload> for ConfirmAvatarUploadHandler {
    async fn handle(&self, cmd: ConfirmAvatarUpload) -> AppResult<()> {
        let mut profile = get_or_create_profile(cmd.customer_id, &self.profile_repo).await?;
        profile.set_avatar(cmd.object_key);
        self.profile_repo.save(&profile).await
    }
}

register_command_handler!(ConfirmAvatarUpload, AppDeps, |deps: &AppDeps| {
    ConfirmAvatarUploadHandler {
        profile_repo: deps.profile_repo.clone(),
    }
});

// ────────────────────────────────────────────────────────────────────────────
// Profile
// ────────────────────────────────────────────────────────────────────────────

pub struct CreateCustomerProfileHandler {
    customer_repo: Arc<dyn CustomerRepository>,
    profile_repo: Arc<dyn CustomerProfileRepository>,
}

#[async_trait]
impl CommandHandler<CreateCustomerProfile> for CreateCustomerProfileHandler {
    async fn handle(&self, cmd: CreateCustomerProfile) -> AppResult<CustomerProfileId> {
        customer_must_exist(cmd.customer_id, &self.customer_repo).await?;
        customer_profile_must_not_exist(cmd.customer_id, &self.profile_repo).await?;

        let id = CustomerProfileId::new();
        let mut profile = CustomerProfile::new(
            id,
            cmd.customer_id,
            cmd.preferred_language,
            cmd.preferred_currency,
        )?;
        profile.update_date_of_birth(cmd.date_of_birth)?;
        profile.update_gender(cmd.gender);
        profile.update_tax_id(cmd.tax_id)?;
        profile.update_company_registration(cmd.company_registration_number)?;
        profile.update_notification_preferences(
            cmd.email_notifications,
            cmd.sms_notifications,
            cmd.push_notifications,
            cmd.marketing_emails,
        );
        self.profile_repo.save(&profile).await?;
        Ok(id)
    }
}

register_command_handler!(CreateCustomerProfile, AppDeps, |deps: &AppDeps| {
    CreateCustomerProfileHandler {
        customer_repo: deps.customer_repo.clone(),
        profile_repo: deps.profile_repo.clone(),
    }
});

pub struct UpdateCustomerProfileHandler {
    profile_repo: Arc<dyn CustomerProfileRepository>,
}

#[async_trait]
impl CommandHandler<UpdateCustomerProfile> for UpdateCustomerProfileHandler {
    async fn handle(&self, cmd: UpdateCustomerProfile) -> AppResult<()> {
        let mut profile = self
            .profile_repo
            .find_by_customer_id(cmd.customer_id)
            .await?
            .ok_or_else(|| AppError::not_found("CustomerProfile", cmd.customer_id.to_string()))?;

        if cmd.set_date_of_birth {
            profile.update_date_of_birth(cmd.date_of_birth)?;
        }
        if cmd.set_gender {
            profile.update_gender(cmd.gender);
        }
        if let Some(lang) = cmd.preferred_language {
            profile.update_preferred_language(lang)?;
        }
        if let Some(curr) = cmd.preferred_currency {
            profile.update_preferred_currency(curr)?;
        }
        if cmd.set_tax_id {
            profile.update_tax_id(cmd.tax_id)?;
        }
        if cmd.set_company_registration_number {
            profile.update_company_registration(cmd.company_registration_number)?;
        }
        self.profile_repo.save(&profile).await
    }
}

register_command_handler!(UpdateCustomerProfile, AppDeps, |deps: &AppDeps| {
    UpdateCustomerProfileHandler {
        profile_repo: deps.profile_repo.clone(),
    }
});

pub struct UpdateNotificationPreferencesHandler {
    profile_repo: Arc<dyn CustomerProfileRepository>,
}

#[async_trait]
impl CommandHandler<UpdateNotificationPreferences> for UpdateNotificationPreferencesHandler {
    async fn handle(&self, cmd: UpdateNotificationPreferences) -> AppResult<()> {
        let mut profile = self
            .profile_repo
            .find_by_customer_id(cmd.customer_id)
            .await?
            .ok_or_else(|| AppError::not_found("CustomerProfile", cmd.customer_id.to_string()))?;
        profile.update_notification_preferences(
            cmd.email_notifications,
            cmd.sms_notifications,
            cmd.push_notifications,
            cmd.marketing_emails,
        );
        self.profile_repo.save(&profile).await
    }
}

register_command_handler!(UpdateNotificationPreferences, AppDeps, |deps: &AppDeps| {
    UpdateNotificationPreferencesHandler {
        profile_repo: deps.profile_repo.clone(),
    }
});

// ────────────────────────────────────────────────────────────────────────────
// KYC
// ────────────────────────────────────────────────────────────────────────────

pub struct SubmitKycDocumentHandler {
    profile_repo: Arc<dyn CustomerProfileRepository>,
}

#[async_trait]
impl CommandHandler<SubmitKycDocument> for SubmitKycDocumentHandler {
    async fn handle(&self, cmd: SubmitKycDocument) -> AppResult<KycSubmissionResult> {
        let mut profile = get_or_create_profile(cmd.customer_id, &self.profile_repo).await?;
        let doc = KycDocument::new(cmd.document_type, cmd.document_number, cmd.file_url)?;
        profile.submit_kyc_document(doc);
        let count = profile.kyc_documents.len() as i32;
        let status = profile.kyc_status.to_string();
        self.profile_repo.save(&profile).await?;
        Ok(KycSubmissionResult {
            kyc_status: status,
            document_count: count,
        })
    }
}

register_command_handler!(SubmitKycDocument, AppDeps, |deps: &AppDeps| {
    SubmitKycDocumentHandler {
        profile_repo: deps.profile_repo.clone(),
    }
});

pub struct RequestKycDocumentUploadUrlHandler {
    customer_repo: Arc<dyn CustomerRepository>,
    blob: Arc<dyn BlobStorage>,
    bucket: String,
    ttl_secs: u64,
}

#[async_trait]
impl CommandHandler<RequestKycDocumentUploadUrl> for RequestKycDocumentUploadUrlHandler {
    async fn handle(&self, cmd: RequestKycDocumentUploadUrl) -> AppResult<UploadUrl> {
        customer_must_exist(cmd.customer_id, &self.customer_repo).await?;
        let object_key = kyc_object_key(cmd.customer_id, &cmd.document_type, &cmd.filename);
        let content_type = if cmd.content_type.trim().is_empty() {
            "application/octet-stream".to_owned()
        } else {
            cmd.content_type
        };
        let ttl = StdDuration::from_secs(self.ttl_secs);
        let link = self
            .blob
            .presigned_put(&self.bucket, &object_key, &content_type, ttl)
            .await?;
        let expires_in_secs = (link.expires_at - Utc::now()).num_seconds().max(0);
        Ok(UploadUrl {
            upload_url: link.url,
            object_key,
            expires_in_secs,
        })
    }
}

register_command_handler!(RequestKycDocumentUploadUrl, AppDeps, |deps: &AppDeps| {
    RequestKycDocumentUploadUrlHandler {
        customer_repo: deps.customer_repo.clone(),
        blob: deps.blob_storage.clone(),
        bucket: deps.blob_bucket.clone(),
        ttl_secs: deps.presign_ttl_secs,
    }
});

pub struct SubmitForKycReviewHandler {
    profile_repo: Arc<dyn CustomerProfileRepository>,
}

#[async_trait]
impl CommandHandler<SubmitForKycReview> for SubmitForKycReviewHandler {
    async fn handle(&self, cmd: SubmitForKycReview) -> AppResult<()> {
        let mut profile = self
            .profile_repo
            .find_by_customer_id(cmd.customer_id)
            .await?
            .ok_or_else(|| AppError::not_found("CustomerProfile", cmd.customer_id.to_string()))?;
        profile.submit_for_kyc_review()?;
        self.profile_repo.save(&profile).await
    }
}

register_command_handler!(SubmitForKycReview, AppDeps, |deps: &AppDeps| {
    SubmitForKycReviewHandler {
        profile_repo: deps.profile_repo.clone(),
    }
});

pub struct VerifyKycHandler {
    profile_repo: Arc<dyn CustomerProfileRepository>,
}

#[async_trait]
impl CommandHandler<VerifyKyc> for VerifyKycHandler {
    async fn handle(&self, cmd: VerifyKyc) -> AppResult<VerifyKycResult> {
        let mut profile = self
            .profile_repo
            .find_by_customer_id(cmd.customer_id)
            .await?
            .ok_or_else(|| AppError::not_found("CustomerProfile", cmd.customer_id.to_string()))?;
        profile.verify_kyc()?;
        let verified_at = profile.kyc_verified_at.unwrap_or_else(Utc::now);
        let status = profile.kyc_status.to_string();
        profile.updated_by = Some(cmd.verified_by.to_string());
        self.profile_repo.save(&profile).await?;
        let _ = profile.drain_events();
        Ok(VerifyKycResult {
            kyc_status: status,
            kyc_verified_at: verified_at,
        })
    }
}

register_command_handler!(VerifyKyc, AppDeps, |deps: &AppDeps| {
    VerifyKycHandler {
        profile_repo: deps.profile_repo.clone(),
    }
});

pub struct RejectKycHandler {
    profile_repo: Arc<dyn CustomerProfileRepository>,
}

#[async_trait]
impl CommandHandler<RejectKyc> for RejectKycHandler {
    async fn handle(&self, cmd: RejectKyc) -> AppResult<String> {
        let mut profile = self
            .profile_repo
            .find_by_customer_id(cmd.customer_id)
            .await?
            .ok_or_else(|| AppError::not_found("CustomerProfile", cmd.customer_id.to_string()))?;
        profile.reject_kyc(cmd.rejection_reason)?;
        let status = profile.kyc_status.to_string();
        profile.updated_by = Some(cmd.rejected_by.to_string());
        self.profile_repo.save(&profile).await?;
        let _ = profile.drain_events();
        Ok(status)
    }
}

register_command_handler!(RejectKyc, AppDeps, |deps: &AppDeps| {
    RejectKycHandler {
        profile_repo: deps.profile_repo.clone(),
    }
});

// ────────────────────────────────────────────────────────────────────────────
// Wishlist
// ────────────────────────────────────────────────────────────────────────────

pub struct AddToWishlistHandler {
    customer_repo: Arc<dyn CustomerRepository>,
    wishlist_repo: Arc<dyn WishlistItemRepository>,
}

#[async_trait]
impl CommandHandler<AddToWishlist> for AddToWishlistHandler {
    async fn handle(&self, cmd: AddToWishlist) -> AppResult<WishlistItemAddedResult> {
        customer_must_exist(cmd.customer_id, &self.customer_repo).await?;
        wishlist_item_must_not_exist(cmd.customer_id, cmd.product_id, &self.wishlist_repo).await?;
        let mut item = WishlistItem::create(
            cmd.customer_id,
            cmd.product_id,
            cmd.product_name,
            cmd.base_price,
        )?;
        let result = WishlistItemAddedResult {
            id: item.id.to_string(),
            product_id: item.product_id,
            added_at: item.added_at,
        };
        self.wishlist_repo.save(&item).await?;
        let _ = item.drain_events();
        Ok(result)
    }
}

register_command_handler!(AddToWishlist, AppDeps, |deps: &AppDeps| {
    AddToWishlistHandler {
        customer_repo: deps.customer_repo.clone(),
        wishlist_repo: deps.wishlist_repo.clone(),
    }
});

pub struct RemoveFromWishlistHandler {
    wishlist_repo: Arc<dyn WishlistItemRepository>,
}

#[async_trait]
impl CommandHandler<RemoveFromWishlist> for RemoveFromWishlistHandler {
    async fn handle(&self, cmd: RemoveFromWishlist) -> AppResult<bool> {
        let Some(mut item) = self
            .wishlist_repo
            .find_by_customer_and_product(cmd.customer_id, cmd.product_id)
            .await?
        else {
            return Ok(false);
        };
        self.wishlist_repo.delete(item.id).await?;
        item.emit_removed();
        let _ = item.drain_events();
        Ok(true)
    }
}

register_command_handler!(RemoveFromWishlist, AppDeps, |deps: &AppDeps| {
    RemoveFromWishlistHandler {
        wishlist_repo: deps.wishlist_repo.clone(),
    }
});

pub struct ClearWishlistHandler {
    wishlist_repo: Arc<dyn WishlistItemRepository>,
}

#[async_trait]
impl CommandHandler<ClearWishlist> for ClearWishlistHandler {
    async fn handle(&self, cmd: ClearWishlist) -> AppResult<()> {
        self.wishlist_repo
            .delete_all_by_customer_id(cmd.customer_id)
            .await
    }
}

register_command_handler!(ClearWishlist, AppDeps, |deps: &AppDeps| {
    ClearWishlistHandler {
        wishlist_repo: deps.wishlist_repo.clone(),
    }
});

// ────────────────────────────────────────────────────────────────────────────
// Queries
// ────────────────────────────────────────────────────────────────────────────

pub struct GetCustomerByIdHandler {
    repo: Arc<dyn CustomerRepository>,
}

#[async_trait]
impl QueryHandler<GetCustomerById> for GetCustomerByIdHandler {
    async fn handle(&self, q: GetCustomerById) -> AppResult<Option<Customer>> {
        self.repo.find_with_children(q.customer_id).await
    }
}

register_query_handler!(GetCustomerById, AppDeps, |deps: &AppDeps| {
    GetCustomerByIdHandler {
        repo: deps.customer_repo.clone(),
    }
});

pub struct GetCustomerByUserIdHandler {
    repo: Arc<dyn CustomerRepository>,
}

#[async_trait]
impl QueryHandler<GetCustomerByUserId> for GetCustomerByUserIdHandler {
    async fn handle(&self, q: GetCustomerByUserId) -> AppResult<Option<Customer>> {
        let Some(c) = self.repo.find_by_user_id(q.user_id).await? else {
            return Ok(None);
        };
        self.repo.find_with_children(c.id).await
    }
}

register_query_handler!(GetCustomerByUserId, AppDeps, |deps: &AppDeps| {
    GetCustomerByUserIdHandler {
        repo: deps.customer_repo.clone(),
    }
});

pub struct ListCustomersHandler {
    repo: Arc<dyn CustomerRepository>,
}

#[async_trait]
impl QueryHandler<ListCustomers> for ListCustomersHandler {
    async fn handle(&self, q: ListCustomers) -> AppResult<Page<Customer>> {
        let filter = CustomerListFilter {
            search: q.search,
            country_code: q.country_code,
            is_membership_active: None,
        };
        self.repo
            .list_paged(PageRequest::new(q.page, q.per_page), filter)
            .await
    }
}

register_query_handler!(ListCustomers, AppDeps, |deps: &AppDeps| {
    ListCustomersHandler {
        repo: deps.customer_repo.clone(),
    }
});

pub struct GetCustomerProfileHandler {
    profile_repo: Arc<dyn CustomerProfileRepository>,
}

#[async_trait]
impl QueryHandler<GetCustomerProfile> for GetCustomerProfileHandler {
    async fn handle(&self, q: GetCustomerProfile) -> AppResult<Option<CustomerProfile>> {
        self.profile_repo.find_by_customer_id(q.customer_id).await
    }
}

register_query_handler!(GetCustomerProfile, AppDeps, |deps: &AppDeps| {
    GetCustomerProfileHandler {
        profile_repo: deps.profile_repo.clone(),
    }
});

pub struct GetCustomerAvatarUrlHandler {
    profile_repo: Arc<dyn CustomerProfileRepository>,
    blob: Arc<dyn BlobStorage>,
    bucket: String,
    ttl_secs: u64,
}

#[async_trait]
impl QueryHandler<GetCustomerAvatarUrl> for GetCustomerAvatarUrlHandler {
    async fn handle(&self, q: GetCustomerAvatarUrl) -> AppResult<AvatarLink> {
        let Some(profile) = self.profile_repo.find_by_customer_id(q.customer_id).await? else {
            return Ok(AvatarLink {
                has_avatar: false,
                avatar_url: None,
                expires_at: None,
            });
        };
        let Some(object_key) = profile.avatar_object_name.as_deref() else {
            return Ok(AvatarLink {
                has_avatar: false,
                avatar_url: None,
                expires_at: None,
            });
        };
        let ttl = StdDuration::from_secs(self.ttl_secs);
        let link = self
            .blob
            .presigned_get(&self.bucket, object_key, ttl)
            .await?;
        let expires_at: DateTime<Utc> = link.expires_at;
        Ok(AvatarLink {
            has_avatar: true,
            avatar_url: Some(link.url),
            expires_at: Some(expires_at),
        })
    }
}

register_query_handler!(GetCustomerAvatarUrl, AppDeps, |deps: &AppDeps| {
    GetCustomerAvatarUrlHandler {
        profile_repo: deps.profile_repo.clone(),
        blob: deps.blob_storage.clone(),
        bucket: deps.blob_bucket.clone(),
        ttl_secs: deps.presign_ttl_secs,
    }
});

pub struct GetWishlistHandler {
    wishlist_repo: Arc<dyn WishlistItemRepository>,
}

#[async_trait]
impl QueryHandler<GetWishlist> for GetWishlistHandler {
    async fn handle(&self, q: GetWishlist) -> AppResult<Vec<WishlistItem>> {
        self.wishlist_repo.list_by_customer_id(q.customer_id).await
    }
}

register_query_handler!(GetWishlist, AppDeps, |deps: &AppDeps| {
    GetWishlistHandler {
        wishlist_repo: deps.wishlist_repo.clone(),
    }
});
