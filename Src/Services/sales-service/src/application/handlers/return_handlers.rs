//! Return command and query handlers, self-registered via Mediator.

use std::sync::Arc;

use async_trait::async_trait;
use ddd_application::{
    register_command_handler, register_query_handler, CommandHandler, QueryHandler,
};
use ddd_shared_kernel::{AppError, AppResult, OutboxMessage, OutboxRepository};
use uuid::Uuid;

use super::super::commands::{AddReturnDetail, CreateReturn, ProcessReturn};
use super::super::deps::AppDeps;
use super::super::dtos::{map_return, ReturnDto};
use super::super::integration_events::{
    ReturnCreatedIntegrationEvent, ReturnProcessedIntegrationEvent,
};
use super::super::queries::{
    GetReturnById, GetReturnsByCustomer, GetReturnsByEmployee, GetReturnsBySale,
};
use crate::domain::entities::Return;
use crate::domain::repositories::{ReturnRepository, SaleRepository};

async fn append_outbox(
    outbox: &Arc<dyn OutboxRepository>,
    aggregate_id: Uuid,
    aggregate_type: &str,
    event_type: &str,
    subject: &str,
    payload: serde_json::Value,
) -> AppResult<()> {
    let msg = OutboxMessage::new(
        aggregate_id.to_string(),
        aggregate_type.to_string(),
        event_type.to_string(),
        subject.to_string(),
        payload,
    );
    outbox.save(&msg).await
}

// ─── CreateReturnHandler ─────────────────────────────────────────────────────

pub struct CreateReturnHandler {
    sale_repo:   Arc<dyn SaleRepository>,
    return_repo: Arc<dyn ReturnRepository>,
    outbox:      Arc<dyn OutboxRepository>,
}

#[async_trait]
impl CommandHandler<CreateReturn> for CreateReturnHandler {
    async fn handle(&self, cmd: CreateReturn) -> AppResult<ReturnDto> {
        // Verify the sale exists.
        let _sale = self
            .sale_repo
            .find_by_id(cmd.sale_id)
            .await?
            .ok_or_else(|| AppError::not_found("Sale", cmd.sale_id.to_string()))?;

        let mut ret = Return::create(cmd.sale_id, cmd.employee_id, cmd.customer_id);
        self.return_repo.save(&mut ret).await?;

        let evt = ReturnCreatedIntegrationEvent {
            return_id:   ret.id.0,
            sale_id:     ret.sale_id.0,
            employee_id: ret.employee_id,
            customer_id: ret.customer_id,
            return_date: ret.return_date.to_rfc3339(),
        };
        append_outbox(
            &self.outbox,
            ret.id.0,
            "Return",
            "ReturnCreated",
            ReturnCreatedIntegrationEvent::TOPIC,
            serde_json::to_value(&evt).map_err(|e| AppError::internal(e.to_string()))?,
        )
        .await?;

        Ok(map_return(&ret))
    }
}

register_command_handler!(CreateReturn, AppDeps, |d: &AppDeps| {
    CreateReturnHandler {
        sale_repo:   d.sale_repo.clone(),
        return_repo: d.return_repo.clone(),
        outbox:      d.outbox.clone(),
    }
});

// ─── AddReturnDetailHandler ──────────────────────────────────────────────────

pub struct AddReturnDetailHandler {
    return_repo: Arc<dyn ReturnRepository>,
}

#[async_trait]
impl CommandHandler<AddReturnDetail> for AddReturnDetailHandler {
    async fn handle(&self, cmd: AddReturnDetail) -> AppResult<()> {
        let mut ret = self
            .return_repo
            .find_with_details(cmd.return_id)
            .await?
            .ok_or_else(|| AppError::not_found("Return", cmd.return_id.to_string()))?;
        ret.add_detail(cmd.product_id, cmd.quantity, cmd.reason, cmd.restock);
        self.return_repo.save(&mut ret).await
    }
}

register_command_handler!(AddReturnDetail, AppDeps, |d: &AppDeps| {
    AddReturnDetailHandler { return_repo: d.return_repo.clone() }
});

// ─── ProcessReturnHandler ────────────────────────────────────────────────────

pub struct ProcessReturnHandler {
    return_repo: Arc<dyn ReturnRepository>,
    outbox:      Arc<dyn OutboxRepository>,
}

#[async_trait]
impl CommandHandler<ProcessReturn> for ProcessReturnHandler {
    async fn handle(&self, cmd: ProcessReturn) -> AppResult<()> {
        let mut ret = self
            .return_repo
            .find_with_details(cmd.return_id)
            .await?
            .ok_or_else(|| AppError::not_found("Return", cmd.return_id.to_string()))?;
        ret.process(cmd.total_refund).map_err(|e| AppError::validation("return", e))?;
        self.return_repo.save(&mut ret).await?;

        let evt = ReturnProcessedIntegrationEvent {
            return_id:    ret.id.0,
            sale_id:      ret.sale_id.0,
            total_refund: ret.total_refund,
        };
        append_outbox(
            &self.outbox,
            ret.id.0,
            "Return",
            "ReturnProcessed",
            ReturnProcessedIntegrationEvent::TOPIC,
            serde_json::to_value(&evt).map_err(|e| AppError::internal(e.to_string()))?,
        )
        .await
    }
}

register_command_handler!(ProcessReturn, AppDeps, |d: &AppDeps| {
    ProcessReturnHandler {
        return_repo: d.return_repo.clone(),
        outbox:      d.outbox.clone(),
    }
});

// ─── Query handlers ──────────────────────────────────────────────────────────

pub struct GetReturnByIdHandler { return_repo: Arc<dyn ReturnRepository> }

#[async_trait]
impl QueryHandler<GetReturnById> for GetReturnByIdHandler {
    async fn handle(&self, q: GetReturnById) -> AppResult<Option<ReturnDto>> {
        Ok(self.return_repo.find_with_details(q.return_id).await?.as_ref().map(map_return))
    }
}

register_query_handler!(GetReturnById, AppDeps, |d: &AppDeps| {
    GetReturnByIdHandler { return_repo: d.return_repo.clone() }
});

pub struct GetReturnsBySaleHandler { return_repo: Arc<dyn ReturnRepository> }

#[async_trait]
impl QueryHandler<GetReturnsBySale> for GetReturnsBySaleHandler {
    async fn handle(&self, q: GetReturnsBySale) -> AppResult<Vec<ReturnDto>> {
        Ok(self.return_repo.get_by_sale(q.sale_id).await?.iter().map(map_return).collect())
    }
}

register_query_handler!(GetReturnsBySale, AppDeps, |d: &AppDeps| {
    GetReturnsBySaleHandler { return_repo: d.return_repo.clone() }
});

pub struct GetReturnsByEmployeeHandler { return_repo: Arc<dyn ReturnRepository> }

#[async_trait]
impl QueryHandler<GetReturnsByEmployee> for GetReturnsByEmployeeHandler {
    async fn handle(&self, q: GetReturnsByEmployee) -> AppResult<Vec<ReturnDto>> {
        Ok(self.return_repo.get_by_employee(q.employee_id, q.from_date, q.to_date).await?.iter().map(map_return).collect())
    }
}

register_query_handler!(GetReturnsByEmployee, AppDeps, |d: &AppDeps| {
    GetReturnsByEmployeeHandler { return_repo: d.return_repo.clone() }
});

pub struct GetReturnsByCustomerHandler { return_repo: Arc<dyn ReturnRepository> }

#[async_trait]
impl QueryHandler<GetReturnsByCustomer> for GetReturnsByCustomerHandler {
    async fn handle(&self, q: GetReturnsByCustomer) -> AppResult<Vec<ReturnDto>> {
        Ok(self.return_repo.get_by_customer(q.customer_id).await?.iter().map(map_return).collect())
    }
}

register_query_handler!(GetReturnsByCustomer, AppDeps, |d: &AppDeps| {
    GetReturnsByCustomerHandler { return_repo: d.return_repo.clone() }
});
