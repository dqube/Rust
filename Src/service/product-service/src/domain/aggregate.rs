use chrono::Utc;
use ddd_domain::{define_aggregate, impl_aggregate, impl_aggregate_events};
use ddd_shared_kernel::AppError;

use super::events::{ProductCreated, ProductDeactivated, ProductId, ProductImageUpdated, ProductStockUpdated};

define_aggregate!(Product, ProductId, {
    pub sku: String,
    pub name: String,
    pub description: String,
    pub price_cents: i64,
    pub stock: u32,
    pub active: bool,
    pub image_url: Option<String>,
});

impl_aggregate!(Product, ProductId);
impl_aggregate_events!(Product);

impl Product {
    pub fn create(
        sku: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        price_cents: i64,
        stock: u32,
    ) -> Result<Self, AppError> {
        let sku = sku.into();
        let name = name.into();
        if sku.trim().is_empty() {
            return Err(AppError::validation("sku", "must not be empty"));
        }
        if name.trim().is_empty() {
            return Err(AppError::validation("name", "must not be empty"));
        }
        if price_cents < 0 {
            return Err(AppError::validation("price", "must be non-negative"));
        }

        let now = Utc::now();
        let id = ProductId::new();
        let mut p = Self {
            id,
            version: 0,
            created_at: now,
            updated_at: now,
            domain_events: Vec::new(),
            sku: sku.clone(),
            name: name.clone(),
            description: description.into(),
            price_cents,
            stock,
            active: true,
            image_url: None,
        };
        p.record_event(ProductCreated {
            product_id: id,
            sku,
            name,
            occurred_at: now,
        });
        Ok(p)
    }

    pub fn update_stock(&mut self, stock: u32) -> Result<(), AppError> {
        if !self.active {
            return Err(AppError::business_rule("Cannot update stock on inactive product"));
        }
        self.stock = stock;
        self.updated_at = Utc::now();
        self.record_event(ProductStockUpdated {
            product_id: self.id,
            stock,
            occurred_at: self.updated_at,
        });
        Ok(())
    }

    pub fn deactivate(&mut self) -> Result<(), AppError> {
        if !self.active {
            return Err(AppError::business_rule("Product is already inactive"));
        }
        self.active = false;
        self.updated_at = Utc::now();
        self.record_event(ProductDeactivated {
            product_id: self.id,
            occurred_at: self.updated_at,
        });
        Ok(())
    }

    pub fn update_image(&mut self, image_url: impl Into<String>) -> Result<(), AppError> {
        if !self.active {
            return Err(AppError::business_rule("Cannot set image on inactive product"));
        }
        let url = image_url.into();
        self.image_url = Some(url.clone());
        self.updated_at = Utc::now();
        self.record_event(ProductImageUpdated {
            product_id: self.id,
            image_url: url,
            occurred_at: self.updated_at,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_product() {
        let p = Product::create("SKU-1", "Widget", "desc", 1999, 10).unwrap();
        assert_eq!(p.sku, "SKU-1");
        assert!(p.active);
        assert_eq!(p.domain_events.len(), 1);
    }

    #[test]
    fn create_empty_sku_fails() {
        assert!(Product::create("", "Widget", "", 100, 0).is_err());
    }

    #[test]
    fn update_stock() {
        let mut p = Product::create("SKU-1", "Widget", "", 100, 5).unwrap();
        p.update_stock(20).unwrap();
        assert_eq!(p.stock, 20);
    }

    #[test]
    fn deactivate_blocks_stock_update() {
        let mut p = Product::create("SKU-1", "Widget", "", 100, 5).unwrap();
        p.deactivate().unwrap();
        assert!(p.update_stock(10).is_err());
    }

    #[test]
    fn double_deactivate_fails() {
        let mut p = Product::create("SKU-1", "Widget", "", 100, 5).unwrap();
        p.deactivate().unwrap();
        assert!(p.deactivate().is_err());
    }
}
