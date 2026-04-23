use chrono::{DateTime, Utc};
use uuid::Uuid;

use ddd_shared_kernel::AppError;

use crate::domain::ids::CategoryId;

#[derive(Debug, Clone)]
pub struct ProductCategory {
    pub id:                 CategoryId,  // SERIAL — 0 means "not yet persisted"
    pub name:               String,
    pub description:        Option<String>,
    pub slug:               Option<String>,
    pub parent_category_id: Option<i32>,
    pub image_url:          Option<String>,
    pub is_active:          bool,
    pub created_at:         DateTime<Utc>,
    pub created_by:         Option<Uuid>,
    pub updated_at:         Option<DateTime<Utc>>,
    pub updated_by:         Option<Uuid>,
}

impl ProductCategory {
    pub fn create(
        name:               String,
        description:        Option<String>,
        parent_category_id: Option<i32>,
    ) -> Result<Self, AppError> {
        if name.trim().is_empty() {
            return Err(AppError::validation("name", "Category name cannot be empty"));
        }
        let slug = slugify(&name);
        Ok(Self {
            id: CategoryId(0),
            name,
            description,
            slug: Some(slug),
            parent_category_id,
            image_url: None,
            is_active: true,
            created_at: Utc::now(),
            created_by: None,
            updated_at: None,
            updated_by: None,
        })
    }

    pub fn update(
        &mut self,
        name:               String,
        description:        Option<String>,
        parent_category_id: Option<i32>,
    ) -> Result<(), AppError> {
        if name.trim().is_empty() {
            return Err(AppError::validation("name", "Category name cannot be empty"));
        }
        self.name = name;
        self.description = description;
        self.parent_category_id = parent_category_id;
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn set_image_url(&mut self, url: String) {
        self.image_url = Some(url);
        self.updated_at = Some(Utc::now());
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
