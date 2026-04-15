//! Command and query handler implementations, self-registered via Mediator.

use std::sync::Arc;

use async_trait::async_trait;
use ddd_application::{
    register_command_handler, register_query_handler,
    CommandHandler, QueryHandler,
};
use ddd_shared_kernel::{AggregateRoot, AppError, AppResult, Page, PageRequest};

use super::commands::{CancelOrder, ConfirmOrder, CreateOrder};
use super::deps::AppDeps;
use super::queries::{GetOrder, ListOrders};
use crate::domain::aggregate::Order;
use crate::domain::events::OrderId;
#[allow(unused_imports)]
use crate::domain::repository::OrderRepository;

// ─── CreateOrderHandler ──────────────────────────────────────────────────────

pub struct CreateOrderHandler {
    repo: Arc<dyn OrderRepository>,
}

#[async_trait]
impl CommandHandler<CreateOrder> for CreateOrderHandler {
    async fn handle(&self, cmd: CreateOrder) -> AppResult<OrderId> {
        let order = Order::create(cmd.customer_id, cmd.items)?;
        self.repo.save(&order).await?;
        Ok(*order.id())
    }
}

register_command_handler!(CreateOrder, AppDeps, |deps: &AppDeps| {
    CreateOrderHandler {
        repo: deps.order_repo.clone(),
    }
});

// ─── ConfirmOrderHandler ─────────────────────────────────────────────────────

pub struct ConfirmOrderHandler {
    repo: Arc<dyn OrderRepository>,
}

#[async_trait]
impl CommandHandler<ConfirmOrder> for ConfirmOrderHandler {
    async fn handle(&self, cmd: ConfirmOrder) -> AppResult<()> {
        let mut order = self
            .repo
            .find_by_id(cmd.order_id)
            .await?
            .ok_or_else(|| AppError::not_found("Order", cmd.order_id.to_string()))?;
        order.confirm()?;
        self.repo.update(&order).await
    }
}

register_command_handler!(ConfirmOrder, AppDeps, |deps: &AppDeps| {
    ConfirmOrderHandler {
        repo: deps.order_repo.clone(),
    }
});

// ─── CancelOrderHandler ─────────────────────────────────────────────────────

pub struct CancelOrderHandler {
    repo: Arc<dyn OrderRepository>,
}

#[async_trait]
impl CommandHandler<CancelOrder> for CancelOrderHandler {
    async fn handle(&self, cmd: CancelOrder) -> AppResult<()> {
        let mut order = self
            .repo
            .find_by_id(cmd.order_id)
            .await?
            .ok_or_else(|| AppError::not_found("Order", cmd.order_id.to_string()))?;
        order.cancel(cmd.reason)?;
        self.repo.update(&order).await
    }
}

register_command_handler!(CancelOrder, AppDeps, |deps: &AppDeps| {
    CancelOrderHandler {
        repo: deps.order_repo.clone(),
    }
});

// ─── GetOrderHandler ─────────────────────────────────────────────────────────

pub struct GetOrderHandler {
    repo: Arc<dyn OrderRepository>,
}

#[async_trait]
impl QueryHandler<GetOrder> for GetOrderHandler {
    async fn handle(&self, q: GetOrder) -> AppResult<Option<Order>> {
        self.repo.find_by_id(q.order_id).await
    }
}

register_query_handler!(GetOrder, AppDeps, |deps: &AppDeps| {
    GetOrderHandler {
        repo: deps.order_repo.clone(),
    }
});

// ─── ListOrdersHandler ──────────────────────────────────────────────────────

pub struct ListOrdersHandler {
    repo: Arc<dyn OrderRepository>,
}

#[async_trait]
impl QueryHandler<ListOrders> for ListOrdersHandler {
    async fn handle(&self, q: ListOrders) -> AppResult<Page<Order>> {
        let page_req = PageRequest::new(q.page, q.per_page);
        self.repo.find_all_paginated(page_req).await
    }
}

register_query_handler!(ListOrders, AppDeps, |deps: &AppDeps| {
    ListOrdersHandler {
        repo: deps.order_repo.clone(),
    }
});
