use chrono::{DateTime, Utc};
use ddd_shared_kernel::{declare_id, DomainEvent};
use serde::{Deserialize, Serialize};
use std::any::Any;

declare_id!(ProductId);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCreated {
    pub product_id: ProductId,
    pub sku: String,
    pub name: String,
    pub occurred_at: DateTime<Utc>,
}

impl DomainEvent for ProductCreated {
    fn event_name(&self) -> &'static str {
        "product.created"
    }
    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductStockUpdated {
    pub product_id: ProductId,
    pub stock: u32,
    pub occurred_at: DateTime<Utc>,
}

impl DomainEvent for ProductStockUpdated {
    fn event_name(&self) -> &'static str {
        "product.stock_updated"
    }
    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductDeactivated {
    pub product_id: ProductId,
    pub occurred_at: DateTime<Utc>,
}

impl DomainEvent for ProductDeactivated {
    fn event_name(&self) -> &'static str {
        "product.deactivated"
    }
    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductImageUpdated {
    pub product_id: ProductId,
    pub image_url: String,
    pub occurred_at: DateTime<Utc>,
}

impl DomainEvent for ProductImageUpdated {
    fn event_name(&self) -> &'static str {
        "product.image_updated"
    }
    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
