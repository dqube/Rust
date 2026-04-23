use std::sync::Arc;

use async_trait::async_trait;
use ddd_application::{CommandHandler, QueryHandler, register_command_handler, register_query_handler};
use ddd_shared_kernel::{AppError, AppResult};

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::queries::*;
use crate::domain::entities::PurchaseOrder;
use crate::domain::repositories::{PurchaseOrderRepository, SupplierRepository};

// ── CreatePurchaseOrder ───────────────────────────────────────────────────────

pub struct CreatePurchaseOrderHandler {
    supplier_repo: Arc<dyn SupplierRepository>,
    order_repo:    Arc<dyn PurchaseOrderRepository>,
}

#[async_trait]
impl CommandHandler<CreatePurchaseOrder> for CreatePurchaseOrderHandler {
    async fn handle(&self, cmd: CreatePurchaseOrder) -> AppResult<PurchaseOrder> {
        self.supplier_repo.find_by_id(cmd.supplier_id).await?
            .ok_or_else(|| AppError::not_found("Supplier", cmd.supplier_id.to_string()))?;
        let mut order = PurchaseOrder::create(
            cmd.supplier_id, cmd.store_id, cmd.expected_date,
            cmd.shipping_address_id, cmd.contact_person_id, cmd.created_by,
        );
        for d in cmd.order_details {
            order.add_detail(d.product_id, d.quantity, d.unit_cost, None);
        }
        self.order_repo.save(&order).await?;
        Ok(order)
    }
}

register_command_handler!(CreatePurchaseOrder, AppDeps, |d: &AppDeps| {
    CreatePurchaseOrderHandler {
        supplier_repo: d.supplier_repo.clone(),
        order_repo:    d.order_repo.clone(),
    }
});

// ── SubmitPurchaseOrder ───────────────────────────────────────────────────────

pub struct SubmitPurchaseOrderHandler {
    repo: Arc<dyn PurchaseOrderRepository>,
}

#[async_trait]
impl CommandHandler<SubmitPurchaseOrder> for SubmitPurchaseOrderHandler {
    async fn handle(&self, cmd: SubmitPurchaseOrder) -> AppResult<PurchaseOrder> {
        let mut order = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("PurchaseOrder", cmd.id.to_string()))?;
        order.submit(cmd.updated_by).map_err(|e| AppError::conflict(e))?;
        self.repo.save(&order).await?;
        Ok(order)
    }
}

register_command_handler!(SubmitPurchaseOrder, AppDeps, |d: &AppDeps| {
    SubmitPurchaseOrderHandler { repo: d.order_repo.clone() }
});

// ── CancelPurchaseOrder ───────────────────────────────────────────────────────

pub struct CancelPurchaseOrderHandler {
    repo: Arc<dyn PurchaseOrderRepository>,
}

#[async_trait]
impl CommandHandler<CancelPurchaseOrder> for CancelPurchaseOrderHandler {
    async fn handle(&self, cmd: CancelPurchaseOrder) -> AppResult<PurchaseOrder> {
        let mut order = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("PurchaseOrder", cmd.id.to_string()))?;
        order.cancel(cmd.updated_by).map_err(|e| AppError::conflict(e))?;
        self.repo.save(&order).await?;
        Ok(order)
    }
}

register_command_handler!(CancelPurchaseOrder, AppDeps, |d: &AppDeps| {
    CancelPurchaseOrderHandler { repo: d.order_repo.clone() }
});

// ── GetPurchaseOrder ──────────────────────────────────────────────────────────

pub struct GetPurchaseOrderHandler {
    repo: Arc<dyn PurchaseOrderRepository>,
}

#[async_trait]
impl QueryHandler<GetPurchaseOrder> for GetPurchaseOrderHandler {
    async fn handle(&self, q: GetPurchaseOrder) -> AppResult<Option<PurchaseOrder>> {
        self.repo.find_by_id(q.id).await
    }
}

register_query_handler!(GetPurchaseOrder, AppDeps, |d: &AppDeps| {
    GetPurchaseOrderHandler { repo: d.order_repo.clone() }
});

// ── ListPurchaseOrders ────────────────────────────────────────────────────────

pub struct ListPurchaseOrdersHandler {
    repo: Arc<dyn PurchaseOrderRepository>,
}

#[async_trait]
impl QueryHandler<ListPurchaseOrders> for ListPurchaseOrdersHandler {
    async fn handle(&self, q: ListPurchaseOrders) -> AppResult<Vec<PurchaseOrder>> {
        self.repo.get_filtered(q.supplier_id, q.store_id, q.status.as_deref(), q.from_date, q.to_date).await
    }
}

register_query_handler!(ListPurchaseOrders, AppDeps, |d: &AppDeps| {
    ListPurchaseOrdersHandler { repo: d.order_repo.clone() }
});
