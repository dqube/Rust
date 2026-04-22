//! In-memory implementation of [`OrderRepository`] for development and testing.
//!
//! A real production deployment would use a SeaORM-backed implementation from
//! `ddd-infrastructure`.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use ddd_domain::Repository;
use ddd_shared_kernel::{AggregateRoot, AppError, AppResult, Page, PageRequest};
use tokio::sync::RwLock;

use crate::domain::aggregate::Order;
use crate::domain::events::OrderId;
use crate::domain::repository::OrderRepository;
use crate::domain::value_objects::{Money, OrderItem, OrderStatus};

/// Thread-safe in-memory order store.
#[derive(Debug, Clone)]
struct StoredOrder {
    id: OrderId,
    customer_id: String,
    items: Vec<OrderItem>,
    total_amount: Money,
    status: OrderStatus,
    version: u64,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl StoredOrder {
    fn from_aggregate(order: &Order) -> Self {
        Self {
            id: *order.id(),
            customer_id: order.customer_id.clone(),
            items: order.items.clone(),
            total_amount: order.total_amount.clone(),
            status: order.status.clone(),
            version: order.version(),
            created_at: order.created_at,
            updated_at: order.updated_at(),
        }
    }

    fn to_aggregate(&self) -> Order {
        Order {
            id: self.id,
            version: self.version,
            created_at: self.created_at,
            updated_at: self.updated_at,
            domain_events: Vec::new(),
            customer_id: self.customer_id.clone(),
            items: self.items.clone(),
            total_amount: self.total_amount.clone(),
            status: self.status.clone(),
        }
    }
}

pub struct InMemoryOrderRepository {
    store: Arc<RwLock<HashMap<OrderId, StoredOrder>>>,
}

impl Default for InMemoryOrderRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryOrderRepository {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Repository<Order> for InMemoryOrderRepository {
    async fn find_by_id(&self, id: OrderId) -> AppResult<Option<Order>> {
        let store = self.store.read().await;
        Ok(store.get(&id).map(|s| s.to_aggregate()))
    }

    async fn save(&self, aggregate: &Order) -> AppResult<()> {
        let mut store = self.store.write().await;
        let id = *aggregate.id();
        if store.contains_key(&id) {
            return Err(AppError::conflict(
                format!("Order {} already exists", id),
            ));
        }
        store.insert(id, StoredOrder::from_aggregate(aggregate));
        Ok(())
    }

    async fn update(&self, aggregate: &Order) -> AppResult<()> {
        let mut store = self.store.write().await;
        let id = *aggregate.id();
        if !store.contains_key(&id) {
            return Err(AppError::not_found("Order", id.to_string()));
        }
        store.insert(id, StoredOrder::from_aggregate(aggregate));
        Ok(())
    }

    async fn delete(&self, id: OrderId) -> AppResult<()> {
        let mut store = self.store.write().await;
        store.remove(&id);
        Ok(())
    }

    async fn exists(&self, id: OrderId) -> AppResult<bool> {
        let store = self.store.read().await;
        Ok(store.contains_key(&id))
    }
}

#[async_trait]
impl OrderRepository for InMemoryOrderRepository {
    async fn find_all_paginated(&self, page: PageRequest) -> AppResult<Page<Order>> {
        let store = self.store.read().await;
        let all: Vec<Order> = store.values().map(|s| s.to_aggregate()).collect();
        let total = all.len() as u64;

        let start = ((page.page().saturating_sub(1)) * page.per_page()) as usize;
        let items: Vec<Order> = all.into_iter().skip(start).take(page.per_page() as usize).collect();

        Ok(Page::new(items, total, page.page(), page.per_page()))
    }

    async fn find_by_customer(&self, customer_id: &str) -> AppResult<Vec<Order>> {
        let store = self.store.read().await;
        let results = store
            .values()
            .filter(|s| s.customer_id == customer_id)
            .map(|s| s.to_aggregate())
            .collect();
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::Money;

    fn sample_items() -> Vec<OrderItem> {
        vec![OrderItem::new("SKU-1", 1, Money::from_f64(10.0))]
    }

    #[tokio::test]
    async fn save_and_find() {
        let repo = InMemoryOrderRepository::new();
        let order = Order::create("cust-1", sample_items()).unwrap();
        let id = *order.id();

        repo.save(&order).await.unwrap();
        let found = repo.find_by_id(id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().customer_id, "cust-1");
    }

    #[tokio::test]
    async fn save_duplicate_fails() {
        let repo = InMemoryOrderRepository::new();
        let order = Order::create("cust-1", sample_items()).unwrap();
        repo.save(&order).await.unwrap();
        assert!(repo.save(&order).await.is_err());
    }

    #[tokio::test]
    async fn paginated_find() {
        let repo = InMemoryOrderRepository::new();
        for i in 0..5 {
            let order = Order::create(format!("cust-{i}"), sample_items()).unwrap();
            repo.save(&order).await.unwrap();
        }

        let page = repo
            .find_all_paginated(PageRequest::new(1, 2))
            .await
            .unwrap();
        assert_eq!(page.items().len(), 2);
        assert_eq!(page.total(), 5);
    }
}
