use chrono::Utc;
use uuid::Uuid;

use ddd_domain::{define_aggregate, impl_aggregate, impl_aggregate_events};
use ddd_shared_kernel::AppError;

use crate::domain::ids::BrandId;

define_aggregate!(Brand, BrandId, {
    pub name:        String,
    pub description: Option<String>,
    pub slug:        Option<String>,
    pub logo_url:    Option<String>,
    pub website:     Option<String>,
    pub is_active:   bool,
    pub created_by:  Option<String>,
    pub updated_by:  Option<String>,
});

impl_aggregate!(Brand, BrandId);
impl_aggregate_events!(Brand);

impl Brand {
    pub fn create(
        name:        String,
        description: Option<String>,
        website:     Option<String>,
    ) -> Result<Self, AppError> {
        if name.trim().is_empty() {
            return Err(AppError::validation("name", "Brand name cannot be empty"));
        }
        let now  = Utc::now();
        let slug = slugify(&name);
        Ok(Self {
            id:            BrandId::from_uuid(Uuid::new_v4()),
            version:       0,
            domain_events: Vec::new(),
            created_at:    now,
            updated_at:    now,
            name,
            description,
            slug:          Some(slug),
            logo_url:      None,
            website,
            is_active:     true,
            created_by:    None,
            updated_by:    None,
        })
    }

    pub fn update(
        &mut self,
        name:        String,
        description: Option<String>,
        website:     Option<String>,
    ) -> Result<(), AppError> {
        if name.trim().is_empty() {
            return Err(AppError::validation("name", "Brand name cannot be empty"));
        }
        self.name = name;
        self.description = description;
        self.website = website;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn activate(&mut self) -> Result<(), AppError> {
        if self.is_active {
            return Err(AppError::conflict("Brand is already active"));
        }
        self.is_active = true;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn deactivate(&mut self) -> Result<(), AppError> {
        if !self.is_active {
            return Err(AppError::conflict("Brand is already inactive"));
        }
        self.is_active = false;
        self.updated_at = Utc::now();
        Ok(())
    }
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
