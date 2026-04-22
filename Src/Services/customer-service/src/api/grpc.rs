//! gRPC server implementation for CustomerService.

use std::sync::Arc;

use ddd_api::grpc::error::GrpcErrorExt;
use ddd_application::Mediator;
use ddd_shared_kernel::AppError;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::application::commands::{
    AddCustomerAddress, AddLoyaltyPoints, AddToWishlist, ClearWishlist, ConfirmAvatarUpload,
    CreateCustomer, CreateCustomerProfile, EnsureCustomerExists, RedeemLoyaltyPoints,
    RejectKyc, RemoveCustomerAddress, RemoveFromWishlist, RequestAvatarUploadUrl,
    RequestKycDocumentUploadUrl, SetDefaultCustomerAddress, SubmitForKycReview,
    SubmitKycDocument, UpdateCustomerAddress, UpdateCustomerInfo, UpdateCustomerProfile,
    UpdateNotificationPreferences, VerifyKyc,
};
use crate::application::queries::{
    GetCustomerAvatarUrl, GetCustomerById, GetCustomerByUserId, GetCustomerProfile, GetWishlist,
    ListCustomers,
};
use crate::domain::entities::{Customer, WishlistItem};
use crate::domain::enums::Gender;
use crate::domain::ids::{CustomerAddressId, CustomerId};
use crate::proto::{
    customer_service_server::{CustomerService, CustomerServiceServer},
    AddCustomerAddressRequest, AddCustomerAddressResponse, AddLoyaltyPointsRequest,
    AddLoyaltyPointsResponse, AddToWishlistRequest, AddToWishlistResponse,
    ClearWishlistRequest, ConfirmAvatarUploadRequest, CreateCustomerProfileRequest,
    CreateCustomerProfileResponse, CreateCustomerRequest, CreateCustomerResponse,
    CustomerAddressMessage, CustomerSummary, Empty, EnsureCustomerProfileRequest,
    EnsureCustomerProfileResponse, GetCustomerAvatarUrlRequest, GetCustomerAvatarUrlResponse,
    GetCustomerByUserIdRequest, GetCustomerProfileRequest, GetCustomerProfileResponse,
    GetCustomerRequest, GetCustomerResponse, GetWishlistRequest, GetWishlistResponse,
    ListCustomersRequest, ListCustomersResponse, RedeemLoyaltyPointsRequest,
    RejectKycRequest, RejectKycResponse, RemoveCustomerAddressRequest, RemoveFromWishlistRequest,
    RemoveFromWishlistResponse, RequestAvatarUploadUrlRequest, RequestAvatarUploadUrlResponse,
    RequestKycDocumentUploadUrlRequest, RequestKycDocumentUploadUrlResponse,
    SetDefaultCustomerAddressRequest, SubmitForKycReviewRequest, SubmitKycDocumentRequest,
    SubmitKycDocumentResponse, UpdateCustomerAddressRequest, UpdateCustomerInfoRequest,
    UpdateCustomerProfileRequest, UpdateNotificationPreferencesRequest, VerifyKycRequest,
    VerifyKycResponse, WishlistItemMessage,
};

#[derive(Clone)]
pub struct CustomerGrpcService {
    mediator: Arc<Mediator>,
}

impl CustomerGrpcService {
    pub fn new(mediator: Arc<Mediator>) -> Self {
        Self { mediator }
    }

    pub fn into_server(self) -> CustomerServiceServer<Self> {
        CustomerServiceServer::new(self)
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_customer_id(s: &str) -> Result<CustomerId, AppError> {
    CustomerId::parse_str(s).map_err(|_| AppError::validation("customer_id", "must be a valid UUID"))
}

fn parse_address_id(s: &str) -> Result<CustomerAddressId, AppError> {
    CustomerAddressId::parse_str(s)
        .map_err(|_| AppError::validation("address_id", "must be a valid UUID"))
}

fn parse_uuid(field: &str, s: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(s).map_err(|_| AppError::validation(field, "must be a valid UUID"))
}

fn to_address_message(a: &crate::domain::entities::CustomerAddress) -> CustomerAddressMessage {
    CustomerAddressMessage {
        address_id: a.id.to_string(),
        label: a.label.clone(),
        line1: a.line1.clone(),
        line2: a.line2.clone().unwrap_or_default(),
        city: a.city.clone(),
        state: a.state.clone().unwrap_or_default(),
        postal_code: a.postal_code.clone(),
        country_code: a.country_code.clone(),
        is_default: a.is_primary,
    }
}

fn to_customer_summary(c: &Customer) -> CustomerSummary {
    CustomerSummary {
        customer_id: c.id.to_string(),
        user_id: c.user_id.to_string(),
        first_name: c.first_name.clone(),
        last_name: c.last_name.clone(),
        email: c.email.clone().unwrap_or_default(),
        membership_number: c.membership_number.clone(),
        country_code: c.country_code.clone(),
        loyalty_points: c.loyalty_points,
        is_membership_active: c.is_membership_active(),
        phone: c.primary_phone().unwrap_or("").to_string(),
        addresses: c.addresses.iter().map(to_address_message).collect(),
    }
}

fn to_wishlist_message(i: &WishlistItem) -> WishlistItemMessage {
    WishlistItemMessage {
        id: i.id.to_string(),
        customer_id: i.customer_id.to_string(),
        product_id: i.product_id.to_string(),
        product_name: i.product_name.clone(),
        base_price: i.base_price,
        added_at: i.added_at.to_rfc3339(),
    }
}

fn parse_opt_gender(has_gender: bool, value: i32) -> Option<Gender> {
    if !has_gender {
        return None;
    }
    match value {
        1 => Some(Gender::Male),
        2 => Some(Gender::Female),
        3 => Some(Gender::NonBinary),
        4 => Some(Gender::PreferNotToSay),
        _ => None,
    }
}

fn parse_opt_date(s: &str) -> Result<Option<chrono::DateTime<chrono::Utc>>, AppError> {
    if s.is_empty() {
        return Ok(None);
    }
    let date = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| AppError::validation("date_of_birth", "must be YYYY-MM-DD"))?;
    Ok(Some(date.and_hms_opt(0, 0, 0).unwrap().and_utc()))
}

// ── RPC implementations ───────────────────────────────────────────────────────

#[tonic::async_trait]
impl CustomerService for CustomerGrpcService {
    // ── Customer CRUD ────────────────────────────────────────────────────────

    async fn create_customer(
        &self,
        req: Request<CreateCustomerRequest>,
    ) -> Result<Response<CreateCustomerResponse>, Status> {
        let r = req.into_inner();
        let user_id = if r.user_id.is_empty() {
            None
        } else {
            Some(parse_uuid("user_id", &r.user_id).map_err(|e| e.to_grpc_status())?)
        };
        let result = self
            .mediator
            .send(CreateCustomer {
                user_id,
                first_name: r.first_name,
                last_name: r.last_name,
                country_code: r.country_code,
                email: if r.email.is_empty() { None } else { Some(r.email) },
                membership_number: None,
                join_date: None,
                expiry_date: None,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CreateCustomerResponse {
            customer_id: result.customer_id.to_string(),
            membership_number: result.membership_number,
        }))
    }

    async fn get_customer(
        &self,
        req: Request<GetCustomerRequest>,
    ) -> Result<Response<GetCustomerResponse>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let customer = self
            .mediator
            .query(GetCustomerById { customer_id: cid })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetCustomerResponse {
            found: customer.is_some(),
            customer: customer.as_ref().map(to_customer_summary),
        }))
    }

    async fn get_customer_by_user_id(
        &self,
        req: Request<GetCustomerByUserIdRequest>,
    ) -> Result<Response<GetCustomerResponse>, Status> {
        let r = req.into_inner();
        let uid = parse_uuid("user_id", &r.user_id).map_err(|e| e.to_grpc_status())?;
        let customer = self
            .mediator
            .query(GetCustomerByUserId { user_id: uid })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetCustomerResponse {
            found: customer.is_some(),
            customer: customer.as_ref().map(to_customer_summary),
        }))
    }

    async fn list_customers(
        &self,
        req: Request<ListCustomersRequest>,
    ) -> Result<Response<ListCustomersResponse>, Status> {
        let r = req.into_inner();
        let page = self
            .mediator
            .query(ListCustomers {
                page: r.page.max(1),
                per_page: r.per_page.clamp(1, 100),
                search: if r.search.is_empty() { None } else { Some(r.search) },
                country_code: if r.country_code.is_empty() {
                    None
                } else {
                    Some(r.country_code)
                },
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ListCustomersResponse {
            items: page.items.iter().map(to_customer_summary).collect(),
            total: page.total,
            page: page.page,
            per_page: page.per_page,
            total_pages: page.total_pages,
        }))
    }

    async fn update_customer_info(
        &self,
        req: Request<UpdateCustomerInfoRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        self.mediator
            .send(UpdateCustomerInfo {
                customer_id: cid,
                first_name: r.first_name,
                last_name: r.last_name,
                phone: if r.phone.is_empty() { None } else { Some(r.phone) },
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }

    async fn ensure_customer_profile(
        &self,
        req: Request<EnsureCustomerProfileRequest>,
    ) -> Result<Response<EnsureCustomerProfileResponse>, Status> {
        let r = req.into_inner();
        let uid = parse_uuid("user_id", &r.user_id).map_err(|e| e.to_grpc_status())?;
        let cid = self
            .mediator
            .send(EnsureCustomerExists {
                user_id: uid,
                first_name: r.first_name,
                last_name: r.last_name,
                email: if r.email.is_empty() { None } else { Some(r.email) },
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(EnsureCustomerProfileResponse {
            customer_id: cid.to_string(),
        }))
    }

    // ── Loyalty ──────────────────────────────────────────────────────────────

    async fn add_loyalty_points(
        &self,
        req: Request<AddLoyaltyPointsRequest>,
    ) -> Result<Response<AddLoyaltyPointsResponse>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let total = self
            .mediator
            .send(AddLoyaltyPoints {
                customer_id: cid,
                points: r.points,
                reason: r.reason,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(AddLoyaltyPointsResponse {
            total_points: total,
        }))
    }

    async fn redeem_loyalty_points(
        &self,
        req: Request<RedeemLoyaltyPointsRequest>,
    ) -> Result<Response<AddLoyaltyPointsResponse>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let total = self
            .mediator
            .send(RedeemLoyaltyPoints {
                customer_id: cid,
                points: r.points,
                reason: r.reason,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(AddLoyaltyPointsResponse {
            total_points: total,
        }))
    }

    // ── Addresses ────────────────────────────────────────────────────────────

    async fn add_customer_address(
        &self,
        req: Request<AddCustomerAddressRequest>,
    ) -> Result<Response<AddCustomerAddressResponse>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let aid = self
            .mediator
            .send(AddCustomerAddress {
                customer_id: cid,
                label: r.label,
                line1: r.line1,
                line2: if r.line2.is_empty() { None } else { Some(r.line2) },
                city: r.city,
                state: if r.state.is_empty() { None } else { Some(r.state) },
                postal_code: r.postal_code,
                country_code: r.country_code,
                is_default: r.is_default,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(AddCustomerAddressResponse {
            address_id: aid.to_string(),
        }))
    }

    async fn update_customer_address(
        &self,
        req: Request<UpdateCustomerAddressRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let aid = parse_address_id(&r.address_id).map_err(|e| e.to_grpc_status())?;
        self.mediator
            .send(UpdateCustomerAddress {
                customer_id: cid,
                address_id: aid,
                label: r.label,
                line1: r.line1,
                line2: if r.line2.is_empty() { None } else { Some(r.line2) },
                city: r.city,
                state: if r.state.is_empty() { None } else { Some(r.state) },
                postal_code: r.postal_code,
                country_code: r.country_code,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }

    async fn remove_customer_address(
        &self,
        req: Request<RemoveCustomerAddressRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let aid = parse_address_id(&r.address_id).map_err(|e| e.to_grpc_status())?;
        self.mediator
            .send(RemoveCustomerAddress {
                customer_id: cid,
                address_id: aid,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }

    async fn set_default_customer_address(
        &self,
        req: Request<SetDefaultCustomerAddressRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let aid = parse_address_id(&r.address_id).map_err(|e| e.to_grpc_status())?;
        self.mediator
            .send(SetDefaultCustomerAddress {
                customer_id: cid,
                address_id: aid,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }

    // ── Avatar ───────────────────────────────────────────────────────────────

    async fn request_avatar_upload_url(
        &self,
        req: Request<RequestAvatarUploadUrlRequest>,
    ) -> Result<Response<RequestAvatarUploadUrlResponse>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let url = self
            .mediator
            .send(RequestAvatarUploadUrl {
                customer_id: cid,
                filename: r.filename,
                content_type: r.content_type,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(RequestAvatarUploadUrlResponse {
            upload_url: url.upload_url,
            object_key: url.object_key,
            expires_in_secs: url.expires_in_secs,
        }))
    }

    async fn confirm_avatar_upload(
        &self,
        req: Request<ConfirmAvatarUploadRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        self.mediator
            .send(ConfirmAvatarUpload {
                customer_id: cid,
                object_key: r.object_key,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }

    async fn get_customer_avatar_url(
        &self,
        req: Request<GetCustomerAvatarUrlRequest>,
    ) -> Result<Response<GetCustomerAvatarUrlResponse>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let link = self
            .mediator
            .query(GetCustomerAvatarUrl { customer_id: cid })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetCustomerAvatarUrlResponse {
            has_avatar: link.has_avatar,
            avatar_url: link.avatar_url.unwrap_or_default(),
        }))
    }

    // ── Profile ──────────────────────────────────────────────────────────────

    async fn create_customer_profile(
        &self,
        req: Request<CreateCustomerProfileRequest>,
    ) -> Result<Response<CreateCustomerProfileResponse>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let dob = parse_opt_date(&r.date_of_birth).map_err(|e| e.to_grpc_status())?;
        let gender = parse_opt_gender(r.has_gender, r.gender);
        let pid = self
            .mediator
            .send(CreateCustomerProfile {
                customer_id: cid,
                preferred_language: r.preferred_language,
                preferred_currency: r.preferred_currency,
                date_of_birth: dob,
                gender,
                tax_id: if r.tax_id.is_empty() { None } else { Some(r.tax_id) },
                company_registration_number: if r.company_registration_number.is_empty() {
                    None
                } else {
                    Some(r.company_registration_number)
                },
                email_notifications: r.email_notifications,
                sms_notifications: r.sms_notifications,
                push_notifications: r.push_notifications,
                marketing_emails: r.marketing_emails,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CreateCustomerProfileResponse {
            profile_id: pid.to_string(),
            customer_id: cid.to_string(),
        }))
    }

    async fn get_customer_profile(
        &self,
        req: Request<GetCustomerProfileRequest>,
    ) -> Result<Response<GetCustomerProfileResponse>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let profile = self
            .mediator
            .query(GetCustomerProfile { customer_id: cid })
            .await
            .map_err(|e| e.to_grpc_status())?;
        match profile {
            None => Ok(Response::new(GetCustomerProfileResponse {
                found: false,
                ..Default::default()
            })),
            Some(p) => Ok(Response::new(GetCustomerProfileResponse {
                found: true,
                profile_id: p.id.to_string(),
                customer_id: p.customer_id.to_string(),
                kyc_status: p.kyc_status.to_string(),
                preferred_language: p.preferred_language.clone(),
                preferred_currency: p.preferred_currency.clone(),
                kyc_document_count: p.kyc_documents.len() as i32,
            })),
        }
    }

    async fn update_customer_profile(
        &self,
        req: Request<UpdateCustomerProfileRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let dob = parse_opt_date(&r.date_of_birth).map_err(|e| e.to_grpc_status())?;
        let gender = parse_opt_gender(r.has_gender, r.gender);
        self.mediator
            .send(UpdateCustomerProfile {
                customer_id: cid,
                date_of_birth: dob,
                set_date_of_birth: !r.date_of_birth.is_empty(),
                gender,
                set_gender: r.has_gender,
                preferred_language: if r.preferred_language.is_empty() {
                    None
                } else {
                    Some(r.preferred_language)
                },
                preferred_currency: if r.preferred_currency.is_empty() {
                    None
                } else {
                    Some(r.preferred_currency)
                },
                tax_id: if r.tax_id.is_empty() { None } else { Some(r.tax_id) },
                set_tax_id: !r.tax_id.is_empty(),
                company_registration_number: if r.company_registration_number.is_empty() {
                    None
                } else {
                    Some(r.company_registration_number)
                },
                set_company_registration_number: !r.company_registration_number.is_empty(),
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }

    async fn update_notification_preferences(
        &self,
        req: Request<UpdateNotificationPreferencesRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        self.mediator
            .send(UpdateNotificationPreferences {
                customer_id: cid,
                email_notifications: r.email_notifications,
                sms_notifications: r.sms_notifications,
                push_notifications: r.push_notifications,
                marketing_emails: r.marketing_emails,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }

    // ── KYC ──────────────────────────────────────────────────────────────────

    async fn submit_kyc_document(
        &self,
        req: Request<SubmitKycDocumentRequest>,
    ) -> Result<Response<SubmitKycDocumentResponse>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let result = self
            .mediator
            .send(SubmitKycDocument {
                customer_id: cid,
                document_type: r.document_type,
                document_number: r.document_number,
                file_url: r.file_url,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(SubmitKycDocumentResponse {
            kyc_status: result.kyc_status,
            document_count: result.document_count,
        }))
    }

    async fn request_kyc_document_upload_url(
        &self,
        req: Request<RequestKycDocumentUploadUrlRequest>,
    ) -> Result<Response<RequestKycDocumentUploadUrlResponse>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let url = self
            .mediator
            .send(RequestKycDocumentUploadUrl {
                customer_id: cid,
                document_type: r.document_type,
                filename: r.filename,
                content_type: r.content_type,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(RequestKycDocumentUploadUrlResponse {
            upload_url: url.upload_url,
            object_key: url.object_key,
            expires_in_secs: url.expires_in_secs,
        }))
    }

    async fn submit_for_kyc_review(
        &self,
        req: Request<SubmitForKycReviewRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        self.mediator
            .send(SubmitForKycReview { customer_id: cid })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }

    async fn verify_kyc(
        &self,
        req: Request<VerifyKycRequest>,
    ) -> Result<Response<VerifyKycResponse>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let verified_by = parse_uuid("verified_by", &r.verified_by).map_err(|e| e.to_grpc_status())?;
        let result = self
            .mediator
            .send(VerifyKyc {
                customer_id: cid,
                verified_by,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(VerifyKycResponse {
            kyc_status: result.kyc_status,
            kyc_verified_at: result.kyc_verified_at.to_rfc3339(),
        }))
    }

    async fn reject_kyc(
        &self,
        req: Request<RejectKycRequest>,
    ) -> Result<Response<RejectKycResponse>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let rejected_by =
            parse_uuid("rejected_by", &r.rejected_by).map_err(|e| e.to_grpc_status())?;
        let status = self
            .mediator
            .send(RejectKyc {
                customer_id: cid,
                rejection_reason: r.rejection_reason,
                rejected_by,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(RejectKycResponse { kyc_status: status }))
    }

    // ── Wishlist ─────────────────────────────────────────────────────────────

    async fn get_wishlist(
        &self,
        req: Request<GetWishlistRequest>,
    ) -> Result<Response<GetWishlistResponse>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let items = self
            .mediator
            .query(GetWishlist { customer_id: cid })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetWishlistResponse {
            items: items.iter().map(to_wishlist_message).collect(),
        }))
    }

    async fn add_to_wishlist(
        &self,
        req: Request<AddToWishlistRequest>,
    ) -> Result<Response<AddToWishlistResponse>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let pid = parse_uuid("product_id", &r.product_id).map_err(|e| e.to_grpc_status())?;
        let result = self
            .mediator
            .send(AddToWishlist {
                customer_id: cid,
                product_id: pid,
                product_name: r.product_name,
                base_price: r.base_price,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(AddToWishlistResponse {
            id: result.id,
            product_id: result.product_id.to_string(),
            added_at: result.added_at.to_rfc3339(),
        }))
    }

    async fn remove_from_wishlist(
        &self,
        req: Request<RemoveFromWishlistRequest>,
    ) -> Result<Response<RemoveFromWishlistResponse>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        let pid = parse_uuid("product_id", &r.product_id).map_err(|e| e.to_grpc_status())?;
        let removed = self
            .mediator
            .send(RemoveFromWishlist {
                customer_id: cid,
                product_id: pid,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(RemoveFromWishlistResponse { removed }))
    }

    async fn clear_wishlist(
        &self,
        req: Request<ClearWishlistRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let cid = parse_customer_id(&r.customer_id).map_err(|e| e.to_grpc_status())?;
        self.mediator
            .send(ClearWishlist { customer_id: cid })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
}
