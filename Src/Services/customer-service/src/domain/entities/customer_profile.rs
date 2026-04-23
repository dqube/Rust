use chrono::{Datelike, Utc};
use serde::{Deserialize, Serialize};

use ddd_domain::{define_aggregate, impl_aggregate, impl_aggregate_events};
use ddd_shared_kernel::{AppError, AppResult, DomainEvent};

use crate::domain::enums::{Gender, KycStatus};
use crate::domain::events::{KycRejected, KycVerified};
use crate::domain::ids::{CustomerId, CustomerProfileId};

// ── KycDocument ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycDocument {
    pub document_type:     String,
    pub document_number:   String,
    pub file_url:          String,
    pub uploaded_at:       chrono::DateTime<Utc>,
    pub verified_at:       Option<chrono::DateTime<Utc>>,
    pub rejection_reason:  Option<String>,
}

impl KycDocument {
    pub fn new(
        document_type:   String,
        document_number: String,
        file_url:        String,
    ) -> AppResult<Self> {
        if document_type.trim().is_empty() {
            return Err(AppError::validation("document_type", "must not be empty"));
        }
        if document_number.trim().is_empty() {
            return Err(AppError::validation("document_number", "must not be empty"));
        }
        if file_url.trim().is_empty() {
            return Err(AppError::validation("file_url", "must not be empty"));
        }
        Ok(Self {
            document_type,
            document_number,
            file_url,
            uploaded_at: Utc::now(),
            verified_at: None,
            rejection_reason: None,
        })
    }

    pub fn mark_as_verified(&mut self) {
        self.verified_at = Some(Utc::now());
        self.rejection_reason = None;
    }

    pub fn mark_as_rejected(&mut self, reason: String) {
        self.rejection_reason = Some(reason);
        self.verified_at = None;
    }
}

// ── CustomerProfile ───────────────────────────────────────────────────────────

define_aggregate!(CustomerProfile, CustomerProfileId, {
    pub customer_id:                   CustomerId,
    pub date_of_birth:                 Option<chrono::DateTime<Utc>>,
    pub gender:                        Option<Gender>,
    pub preferred_language:            String,
    pub preferred_currency:            String,
    pub tax_id:                        Option<String>,
    pub company_registration_number:   Option<String>,
    pub email_notifications:           bool,
    pub sms_notifications:             bool,
    pub push_notifications:            bool,
    pub marketing_emails:              bool,
    pub kyc_status:                    KycStatus,
    pub kyc_verified_at:               Option<chrono::DateTime<Utc>>,
    pub kyc_documents:                 Vec<KycDocument>,
    pub avatar_object_name:            Option<String>,
    pub created_by:                    Option<String>,
    pub updated_by:                    Option<String>,
});

impl_aggregate!(CustomerProfile, CustomerProfileId);
impl_aggregate_events!(CustomerProfile);

impl CustomerProfile {
    pub fn new(
        id:                  CustomerProfileId,
        customer_id:         CustomerId,
        preferred_language:  String,
        preferred_currency:  String,
    ) -> AppResult<Self> {
        let lang = if preferred_language.is_empty() { "en".to_owned() } else { preferred_language };
        let curr = if preferred_currency.is_empty() { "USD".to_owned() } else { preferred_currency };
        if lang.len() != 2 {
            return Err(AppError::validation("preferred_language", "must be 2 characters (ISO 639-1)"));
        }
        if curr.len() != 3 {
            return Err(AppError::validation("preferred_currency", "must be 3 characters (ISO 4217)"));
        }
        let now = Utc::now();
        Ok(Self {
            id,
            version: 0,
            domain_events: Vec::new(),
            created_at: now,
            updated_at: now,
            customer_id,
            date_of_birth: None,
            gender: None,
            preferred_language: lang,
            preferred_currency: curr,
            tax_id: None,
            company_registration_number: None,
            email_notifications: true,
            sms_notifications: true,
            push_notifications: true,
            marketing_emails: false,
            kyc_status: KycStatus::Pending,
            kyc_verified_at: None,
            kyc_documents: Vec::new(),
            avatar_object_name: None,
            created_by: None,
            updated_by: None,
        })
    }

    pub fn update_date_of_birth(&mut self, dob: Option<chrono::DateTime<Utc>>) -> AppResult<()> {
        if let Some(dob) = dob {
            Self::validate_dob(dob)?;
        }
        self.date_of_birth = dob;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn update_gender(&mut self, gender: Option<Gender>) {
        self.gender = gender;
        self.updated_at = Utc::now();
    }

    pub fn update_preferred_language(&mut self, lang: String) -> AppResult<()> {
        if lang.len() != 2 {
            return Err(AppError::validation("preferred_language", "must be 2 characters"));
        }
        self.preferred_language = lang;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn update_preferred_currency(&mut self, curr: String) -> AppResult<()> {
        if curr.len() != 3 {
            return Err(AppError::validation("preferred_currency", "must be 3 characters"));
        }
        self.preferred_currency = curr;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn update_tax_id(&mut self, tax_id: Option<String>) -> AppResult<()> {
        if let Some(ref id) = tax_id {
            if id.len() > 50 {
                return Err(AppError::validation("tax_id", "must be at most 50 characters"));
            }
        }
        self.tax_id = tax_id;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn update_company_registration(&mut self, reg: Option<String>) -> AppResult<()> {
        if let Some(ref r) = reg {
            if r.len() > 100 {
                return Err(AppError::validation(
                    "company_registration_number",
                    "must be at most 100 characters",
                ));
            }
        }
        self.company_registration_number = reg;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn update_notification_preferences(
        &mut self,
        email:     bool,
        sms:       bool,
        push:      bool,
        marketing: bool,
    ) {
        self.email_notifications = email;
        self.sms_notifications = sms;
        self.push_notifications = push;
        self.marketing_emails = marketing;
        self.updated_at = Utc::now();
    }

    pub fn submit_kyc_document(&mut self, doc: KycDocument) {
        self.kyc_documents.push(doc);
        self.updated_at = Utc::now();
    }

    pub fn submit_for_kyc_review(&mut self) -> AppResult<()> {
        if self.kyc_documents.is_empty() {
            return Err(AppError::conflict("cannot submit for KYC review without documents"));
        }
        if self.kyc_status == KycStatus::Verified {
            return Err(AppError::conflict("KYC is already verified"));
        }
        self.kyc_status = KycStatus::Submitted;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn verify_kyc(&mut self) -> AppResult<()> {
        if self.kyc_status == KycStatus::Verified {
            return Err(AppError::conflict("KYC is already verified"));
        }
        self.kyc_status = KycStatus::Verified;
        let verified_at = Utc::now();
        self.kyc_verified_at = Some(verified_at);
        self.updated_at = verified_at;
        for doc in &mut self.kyc_documents {
            doc.mark_as_verified();
        }
        self.domain_events.push(Box::new(KycVerified {
            customer_id: self.customer_id,
            verified_at,
            occurred_at: verified_at,
        }));
        Ok(())
    }

    pub fn reject_kyc(&mut self, reason: String) -> AppResult<()> {
        if reason.trim().is_empty() {
            return Err(AppError::validation("rejection_reason", "must not be empty"));
        }
        self.kyc_status = KycStatus::Rejected;
        self.kyc_verified_at = None;
        let now = Utc::now();
        self.updated_at = now;
        for doc in &mut self.kyc_documents {
            doc.mark_as_rejected(reason.clone());
        }
        self.domain_events.push(Box::new(KycRejected {
            customer_id: self.customer_id,
            rejection_reason: reason,
            occurred_at: now,
        }));
        Ok(())
    }

    pub fn set_avatar(&mut self, object_name: String) {
        self.avatar_object_name = Some(object_name);
        self.updated_at = Utc::now();
    }

    pub fn remove_avatar(&mut self) {
        self.avatar_object_name = None;
        self.updated_at = Utc::now();
    }

    pub fn is_kyc_verified(&self) -> bool {
        self.kyc_status == KycStatus::Verified
    }

    pub fn get_age(&self) -> Option<i32> {
        let dob = self.date_of_birth?;
        let today = Utc::now();
        let mut age = today.year() - dob.year();
        if today.ordinal() < dob.ordinal() {
            age -= 1;
        }
        Some(age)
    }

    pub fn drain_events(&mut self) -> Vec<Box<dyn DomainEvent>> {
        std::mem::take(&mut self.domain_events)
    }

    fn validate_dob(dob: chrono::DateTime<Utc>) -> AppResult<()> {
        if dob >= Utc::now() {
            return Err(AppError::validation("date_of_birth", "must be in the past"));
        }
        let age = Utc::now().year() - dob.year();
        if age < 18 {
            return Err(AppError::validation("date_of_birth", "customer must be at least 18 years old"));
        }
        if age > 120 {
            return Err(AppError::validation("date_of_birth", "invalid"));
        }
        Ok(())
    }
}
