use chrono::{DateTime, Utc};
use uuid::Uuid;

use ddd_shared_kernel::AppError;

use crate::domain::ids::TaxConfigId;

#[derive(Debug, Clone)]
pub struct TaxConfiguration {
    pub id:             TaxConfigId,
    pub name:           String,
    pub code:           String,
    pub tax_type:       String,
    pub location_id:    i32,
    pub category_id:    Option<i32>,
    pub tax_rate:       f64,
    pub is_active:      bool,
    pub effective_date: DateTime<Utc>,
    pub expiry_date:    Option<DateTime<Utc>>,
    pub created_at:     DateTime<Utc>,
    pub created_by:     Option<String>,
    pub updated_at:     Option<DateTime<Utc>>,
    pub updated_by:     Option<String>,
}

impl TaxConfiguration {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        name:           String,
        code:           String,
        tax_type:       String,
        tax_rate:       f64,
        location_id:    i32,
        category_id:    Option<i32>,
        effective_date: DateTime<Utc>,
        expiry_date:    Option<DateTime<Utc>>,
    ) -> Result<Self, AppError> {
        if name.trim().is_empty() {
            return Err(AppError::validation("name", "Tax configuration name cannot be empty"));
        }
        if code.trim().is_empty() {
            return Err(AppError::validation("tax_code", "Tax code cannot be empty"));
        }
        Ok(Self {
            id: TaxConfigId::from_uuid(Uuid::new_v4()),
            name,
            code,
            tax_type,
            location_id,
            category_id,
            tax_rate,
            is_active: true,
            effective_date,
            expiry_date,
            created_at: Utc::now(),
            created_by: None,
            updated_at: None,
            updated_by: None,
        })
    }

    pub fn update(
        &mut self,
        name:           String,
        code:           String,
        tax_type:       String,
        tax_rate:       f64,
        effective_date: DateTime<Utc>,
        expiry_date:    Option<DateTime<Utc>>,
    ) -> Result<(), AppError> {
        if name.trim().is_empty() {
            return Err(AppError::validation("name", "Tax configuration name cannot be empty"));
        }
        if code.trim().is_empty() {
            return Err(AppError::validation("tax_code", "Tax code cannot be empty"));
        }
        self.name = name;
        self.code = code;
        self.tax_type = tax_type;
        self.tax_rate = tax_rate;
        self.effective_date = effective_date;
        self.expiry_date = expiry_date;
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn activate(&mut self) -> Result<(), AppError> {
        if self.is_active {
            return Err(AppError::conflict("Tax configuration is already active"));
        }
        self.is_active = true;
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn deactivate(&mut self) -> Result<(), AppError> {
        if !self.is_active {
            return Err(AppError::conflict("Tax configuration is already inactive"));
        }
        self.is_active = false;
        self.updated_at = Some(Utc::now());
        Ok(())
    }
}
