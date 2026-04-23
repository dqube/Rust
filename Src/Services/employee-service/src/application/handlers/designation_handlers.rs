use std::sync::Arc;

use async_trait::async_trait;
use ddd_application::{CommandHandler, QueryHandler, register_command_handler, register_query_handler};
use ddd_shared_kernel::{AppError, AppResult};

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::queries::*;
use crate::domain::entities::Designation;
use crate::domain::ids::DesignationId;
use crate::domain::repositories::DesignationRepository;

// ── CreateDesignation ─────────────────────────────────────────────────────────

pub struct CreateDesignationHandler {
    repo: Arc<dyn DesignationRepository>,
}

#[async_trait]
impl CommandHandler<CreateDesignation> for CreateDesignationHandler {
    async fn handle(&self, cmd: CreateDesignation) -> AppResult<Designation> {
        if self.repo.name_exists(&cmd.designation_name).await? {
            return Err(AppError::conflict("A designation with this name already exists"));
        }
        let des = Designation::create(cmd.designation_name, cmd.level);
        self.repo.save(&des).await?;
        Ok(des)
    }
}

register_command_handler!(CreateDesignation, AppDeps, |d: &AppDeps| {
    CreateDesignationHandler { repo: d.designation_repo.clone() }
});

// ── UpdateDesignation ─────────────────────────────────────────────────────────

pub struct UpdateDesignationHandler {
    repo: Arc<dyn DesignationRepository>,
}

#[async_trait]
impl CommandHandler<UpdateDesignation> for UpdateDesignationHandler {
    async fn handle(&self, cmd: UpdateDesignation) -> AppResult<Designation> {
        let mut des = self.repo.find_by_id(DesignationId::from_uuid(cmd.id)).await?
            .ok_or_else(|| AppError::not_found("Designation", cmd.id.to_string()))?;
        des.designation_name = cmd.designation_name;
        des.level            = cmd.level;
        des.updated_at       = chrono::Utc::now();
        self.repo.save(&des).await?;
        Ok(des)
    }
}

register_command_handler!(UpdateDesignation, AppDeps, |d: &AppDeps| {
    UpdateDesignationHandler { repo: d.designation_repo.clone() }
});

// ── GetDesignation ────────────────────────────────────────────────────────────

pub struct GetDesignationHandler {
    repo: Arc<dyn DesignationRepository>,
}

#[async_trait]
impl QueryHandler<GetDesignation> for GetDesignationHandler {
    async fn handle(&self, q: GetDesignation) -> AppResult<Option<Designation>> {
        self.repo.find_by_id(DesignationId::from_uuid(q.id)).await
    }
}

register_query_handler!(GetDesignation, AppDeps, |d: &AppDeps| {
    GetDesignationHandler { repo: d.designation_repo.clone() }
});

// ── ListDesignations ──────────────────────────────────────────────────────────

pub struct ListDesignationsHandler {
    repo: Arc<dyn DesignationRepository>,
}

#[async_trait]
impl QueryHandler<ListDesignations> for ListDesignationsHandler {
    async fn handle(&self, _q: ListDesignations) -> AppResult<Vec<Designation>> {
        self.repo.get_all().await
    }
}

register_query_handler!(ListDesignations, AppDeps, |d: &AppDeps| {
    ListDesignationsHandler { repo: d.designation_repo.clone() }
});
