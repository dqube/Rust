use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use ddd_domain::Repository;
use ddd_shared_kernel::{AggregateRoot, AppError, AppResult, Page, PageRequest};
use tokio::sync::RwLock;

use crate::domain::aggregate::Product;
use crate::domain::events::ProductId;
use crate::domain::repository::ProductRepository;

#[derive(Debug, Clone)]
struct Stored {
    id: ProductId,
    sku: String,
    name: String,
    description: String,
    price_cents: i64,
    stock: u32,
    active: bool,
    image_url: Option<String>,
    version: u64,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl Stored {
    fn from_aggregate(p: &Product) -> Self {
        Self {
            id: *p.id(),
            sku: p.sku.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            price_cents: p.price_cents,
            stock: p.stock,
            active: p.active,
            image_url: p.image_url.clone(),
            version: p.version(),
            created_at: p.created_at,
            updated_at: p.updated_at(),
        }
    }
    fn to_aggregate(&self) -> Product {
        Product {
            id: self.id,
            version: self.version,
            created_at: self.created_at,
            updated_at: self.updated_at,
            domain_events: Vec::new(),
            sku: self.sku.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            price_cents: self.price_cents,
            stock: self.stock,
            active: self.active,
            image_url: self.image_url.clone(),
        }
    }
}

pub struct InMemoryProductRepository {
    store: Arc<RwLock<HashMap<ProductId, Stored>>>,
}

impl Default for InMemoryProductRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryProductRepository {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Repository<Product> for InMemoryProductRepository {
    async fn find_by_id(&self, id: ProductId) -> AppResult<Option<Product>> {
        Ok(self.store.read().await.get(&id).map(|s| s.to_aggregate()))
    }
    async fn save(&self, aggregate: &Product) -> AppResult<()> {
        let mut s = self.store.write().await;
        let id = *aggregate.id();
        if s.contains_key(&id) {
            return Err(AppError::conflict(format!("Product {} already exists", id)));
        }
        s.insert(id, Stored::from_aggregate(aggregate));
        Ok(())
    }
    async fn update(&self, aggregate: &Product) -> AppResult<()> {
        let mut s = self.store.write().await;
        let id = *aggregate.id();
        if !s.contains_key(&id) {
            return Err(AppError::not_found("Product", id.to_string()));
        }
        s.insert(id, Stored::from_aggregate(aggregate));
        Ok(())
    }
    async fn delete(&self, id: ProductId) -> AppResult<()> {
        self.store.write().await.remove(&id);
        Ok(())
    }
    async fn exists(&self, id: ProductId) -> AppResult<bool> {
        Ok(self.store.read().await.contains_key(&id))
    }
}

#[async_trait]
impl ProductRepository for InMemoryProductRepository {
    async fn find_all_paginated(&self, page: PageRequest) -> AppResult<Page<Product>> {
        let s = self.store.read().await;
        let all: Vec<Product> = s.values().map(|r| r.to_aggregate()).collect();
        let total = all.len() as u64;
        let start = ((page.page().saturating_sub(1)) * page.per_page()) as usize;
        let items: Vec<Product> = all.into_iter().skip(start).take(page.per_page() as usize).collect();
        Ok(Page::new(items, total, page.page(), page.per_page()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn save_and_find() {
        let repo = InMemoryProductRepository::new();
        let p = Product::create("SKU-1", "Widget", "", 100, 5).unwrap();
        let id = *p.id();
        repo.save(&p).await.unwrap();
        assert!(repo.find_by_id(id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn paginated() {
        let repo = InMemoryProductRepository::new();
        for i in 0..4 {
            let p = Product::create(format!("SKU-{i}"), "w", "", 100, 0).unwrap();
            repo.save(&p).await.unwrap();
        }
        let page = repo.find_all_paginated(PageRequest::new(1, 2)).await.unwrap();
        assert_eq!(page.items().len(), 2);
        assert_eq!(page.total(), 4);
    }
}
