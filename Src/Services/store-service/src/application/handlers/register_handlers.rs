use std::sync::Arc;

use async_trait::async_trait;
use ddd_application::{register_command_handler, register_query_handler, CommandHandler, QueryHandler};
use ddd_shared_kernel::{AppError, AppResult, OutboxMessage, OutboxRepository};

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::dtos::{map_register, PagedRegisterDto, RegisterDto};
use crate::application::integration_events::*;
use crate::application::queries::*;
use crate::domain::entities::Register;
use crate::domain::repositories::{RegisterRepository, StoreRepository};

fn outbox_msg(id: i32, topic: &'static str, payload: serde_json::Value) -> OutboxMessage {
    OutboxMessage::new(id.to_string(), "Register", topic, topic, payload)
}

// ── CreateRegisterHandler ─────────────────────────────────────────────────────

pub struct CreateRegisterHandler {
    store_repo:    Arc<dyn StoreRepository>,
    register_repo: Arc<dyn RegisterRepository>,
    outbox:        Arc<dyn OutboxRepository>,
}

#[async_trait]
impl CommandHandler<CreateRegister> for CreateRegisterHandler {
    async fn handle(&self, cmd: CreateRegister) -> AppResult<RegisterDto> {
        self.store_repo.find_by_id(cmd.store_id).await?
            .ok_or_else(|| AppError::not_found("Store", cmd.store_id.to_string()))?;

        if self.register_repo.name_exists_in_store(cmd.store_id, &cmd.name).await? {
            return Err(AppError::conflict(format!(
                "Register '{}' already exists in store {}.", cmd.name, cmd.store_id,
            )));
        }

        let mut reg = Register::create(cmd.store_id, cmd.name)?;
        self.register_repo.save(&mut reg).await?;
        reg.emit_created();

        let evt = RegisterCreatedIntegrationEvent {
            register_id: reg.id.0,
            store_id:    reg.store_id.0,
            name:        reg.name.clone(),
        };
        let payload = serde_json::to_value(&evt).map_err(|e| AppError::internal(e.to_string()))?;
        self.outbox.save(&outbox_msg(reg.id.0, RegisterCreatedIntegrationEvent::TOPIC, payload)).await?;

        let _ = reg.drain_events();
        Ok(map_register(&reg))
    }
}

register_command_handler!(CreateRegister, AppDeps, |d: &AppDeps| {
    CreateRegisterHandler {
        store_repo:    d.store_repo.clone(),
        register_repo: d.register_repo.clone(),
        outbox:        d.outbox.clone(),
    }
});

// ── OpenRegisterHandler ───────────────────────────────────────────────────────

pub struct OpenRegisterHandler {
    register_repo: Arc<dyn RegisterRepository>,
    outbox:        Arc<dyn OutboxRepository>,
}

#[async_trait]
impl CommandHandler<OpenRegister> for OpenRegisterHandler {
    async fn handle(&self, cmd: OpenRegister) -> AppResult<()> {
        let mut reg = self.register_repo.find_by_id(cmd.register_id).await?
            .ok_or_else(|| AppError::not_found("Register", cmd.register_id.to_string()))?;
        reg.open(cmd.starting_cash)?;
        self.register_repo.save(&mut reg).await?;

        let evt = RegisterOpenedIntegrationEvent {
            register_id:   reg.id.0,
            store_id:      reg.store_id.0,
            starting_cash: cmd.starting_cash.to_string(),
        };
        let payload = serde_json::to_value(&evt).map_err(|e| AppError::internal(e.to_string()))?;
        self.outbox.save(&outbox_msg(reg.id.0, RegisterOpenedIntegrationEvent::TOPIC, payload)).await?;
        let _ = reg.drain_events();
        Ok(())
    }
}

register_command_handler!(OpenRegister, AppDeps, |d: &AppDeps| {
    OpenRegisterHandler { register_repo: d.register_repo.clone(), outbox: d.outbox.clone() }
});

// ── CloseRegisterHandler ──────────────────────────────────────────────────────

pub struct CloseRegisterHandler {
    register_repo: Arc<dyn RegisterRepository>,
    outbox:        Arc<dyn OutboxRepository>,
}

#[async_trait]
impl CommandHandler<CloseRegister> for CloseRegisterHandler {
    async fn handle(&self, cmd: CloseRegister) -> AppResult<()> {
        let mut reg = self.register_repo.find_by_id(cmd.register_id).await?
            .ok_or_else(|| AppError::not_found("Register", cmd.register_id.to_string()))?;
        let variance = reg.close(cmd.ending_cash)?;
        self.register_repo.save(&mut reg).await?;

        let evt = RegisterClosedIntegrationEvent {
            register_id: reg.id.0,
            store_id:    reg.store_id.0,
            ending_cash: cmd.ending_cash.to_string(),
            variance:    variance.to_string(),
        };
        let payload = serde_json::to_value(&evt).map_err(|e| AppError::internal(e.to_string()))?;
        self.outbox.save(&outbox_msg(reg.id.0, RegisterClosedIntegrationEvent::TOPIC, payload)).await?;
        let _ = reg.drain_events();
        Ok(())
    }
}

register_command_handler!(CloseRegister, AppDeps, |d: &AppDeps| {
    CloseRegisterHandler { register_repo: d.register_repo.clone(), outbox: d.outbox.clone() }
});

// ── AddCashToRegisterHandler ──────────────────────────────────────────────────

pub struct AddCashToRegisterHandler { register_repo: Arc<dyn RegisterRepository> }

#[async_trait]
impl CommandHandler<AddCashToRegister> for AddCashToRegisterHandler {
    async fn handle(&self, cmd: AddCashToRegister) -> AppResult<()> {
        let mut reg = self.register_repo.find_by_id(cmd.register_id).await?
            .ok_or_else(|| AppError::not_found("Register", cmd.register_id.to_string()))?;
        reg.add_cash(cmd.amount)?;
        self.register_repo.save(&mut reg).await
    }
}

register_command_handler!(AddCashToRegister, AppDeps, |d: &AppDeps| {
    AddCashToRegisterHandler { register_repo: d.register_repo.clone() }
});

// ── RemoveCashFromRegisterHandler ─────────────────────────────────────────────

pub struct RemoveCashFromRegisterHandler { register_repo: Arc<dyn RegisterRepository> }

#[async_trait]
impl CommandHandler<RemoveCashFromRegister> for RemoveCashFromRegisterHandler {
    async fn handle(&self, cmd: RemoveCashFromRegister) -> AppResult<()> {
        let mut reg = self.register_repo.find_by_id(cmd.register_id).await?
            .ok_or_else(|| AppError::not_found("Register", cmd.register_id.to_string()))?;
        reg.remove_cash(cmd.amount)?;
        self.register_repo.save(&mut reg).await
    }
}

register_command_handler!(RemoveCashFromRegister, AppDeps, |d: &AppDeps| {
    RemoveCashFromRegisterHandler { register_repo: d.register_repo.clone() }
});

// ── UpdateRegisterNameHandler ─────────────────────────────────────────────────

pub struct UpdateRegisterNameHandler { register_repo: Arc<dyn RegisterRepository> }

#[async_trait]
impl CommandHandler<UpdateRegisterName> for UpdateRegisterNameHandler {
    async fn handle(&self, cmd: UpdateRegisterName) -> AppResult<RegisterDto> {
        let mut reg = self.register_repo.find_by_id(cmd.register_id).await?
            .ok_or_else(|| AppError::not_found("Register", cmd.register_id.to_string()))?;
        reg.update_name(cmd.new_name)?;
        self.register_repo.save(&mut reg).await?;
        let _ = reg.drain_events();
        Ok(map_register(&reg))
    }
}

register_command_handler!(UpdateRegisterName, AppDeps, |d: &AppDeps| {
    UpdateRegisterNameHandler { register_repo: d.register_repo.clone() }
});

// ── Query handlers ────────────────────────────────────────────────────────────

pub struct GetRegisterHandler { register_repo: Arc<dyn RegisterRepository> }

#[async_trait]
impl QueryHandler<GetRegister> for GetRegisterHandler {
    async fn handle(&self, q: GetRegister) -> AppResult<Option<RegisterDto>> {
        Ok(self.register_repo.find_by_id(q.register_id).await?.map(|r| map_register(&r)))
    }
}

register_query_handler!(GetRegister, AppDeps, |d: &AppDeps| {
    GetRegisterHandler { register_repo: d.register_repo.clone() }
});

pub struct ListRegistersHandler { register_repo: Arc<dyn RegisterRepository> }

#[async_trait]
impl QueryHandler<ListRegisters> for ListRegistersHandler {
    async fn handle(&self, q: ListRegisters) -> AppResult<PagedRegisterDto> {
        let page      = if q.page < 1 { 1 } else { q.page };
        let page_size = if q.page_size < 1 { 20 } else { q.page_size };
        let result = self.register_repo.get_paged(q.store_id, page, page_size).await?;
        Ok(PagedRegisterDto {
            items:     result.items.iter().map(map_register).collect(),
            total:     result.total,
            page:      result.page,
            page_size: result.page_size,
        })
    }
}

register_query_handler!(ListRegisters, AppDeps, |d: &AppDeps| {
    ListRegistersHandler { register_repo: d.register_repo.clone() }
});
