use std::sync::Arc;

use async_trait::async_trait;
use ddd_application::{CommandHandler, QueryHandler, register_command_handler, register_query_handler};
use ddd_shared_kernel::{AppError, AppResult};

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::queries::*;
use crate::domain::entities::TaxConfiguration;
use crate::domain::repositories::TaxConfigRepository;

pub struct CreateTaxConfigurationHandler {
    repo: Arc<dyn TaxConfigRepository>,
}

#[async_trait]
impl CommandHandler<CreateTaxConfiguration> for CreateTaxConfigurationHandler {
    async fn handle(&self, cmd: CreateTaxConfiguration) -> AppResult<TaxConfiguration> {
        let tc = TaxConfiguration::create(
            cmd.name, cmd.code, cmd.tax_type, cmd.tax_rate,
            cmd.location_id, cmd.category_id, cmd.effective_date, cmd.expiry_date,
        )?;
        self.repo.save(&tc).await?;
        Ok(tc)
    }
}

register_command_handler!(CreateTaxConfiguration, AppDeps, |d: &AppDeps| {
    CreateTaxConfigurationHandler { repo: d.tax_repo.clone() }
});

pub struct UpdateTaxConfigurationHandler {
    repo: Arc<dyn TaxConfigRepository>,
}

#[async_trait]
impl CommandHandler<UpdateTaxConfiguration> for UpdateTaxConfigurationHandler {
    async fn handle(&self, cmd: UpdateTaxConfiguration) -> AppResult<TaxConfiguration> {
        let mut tc = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("TaxConfig", cmd.id.to_string()))?;
        tc.update(cmd.name, cmd.code, cmd.tax_type, cmd.tax_rate, cmd.effective_date, cmd.expiry_date)?;
        self.repo.save(&tc).await?;
        Ok(tc)
    }
}

register_command_handler!(UpdateTaxConfiguration, AppDeps, |d: &AppDeps| {
    UpdateTaxConfigurationHandler { repo: d.tax_repo.clone() }
});

pub struct ActivateTaxConfigurationHandler {
    repo: Arc<dyn TaxConfigRepository>,
}

#[async_trait]
impl CommandHandler<ActivateTaxConfiguration> for ActivateTaxConfigurationHandler {
    async fn handle(&self, cmd: ActivateTaxConfiguration) -> AppResult<TaxConfiguration> {
        let mut tc = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("TaxConfig", cmd.id.to_string()))?;
        tc.activate()?;
        self.repo.save(&tc).await?;
        Ok(tc)
    }
}

register_command_handler!(ActivateTaxConfiguration, AppDeps, |d: &AppDeps| {
    ActivateTaxConfigurationHandler { repo: d.tax_repo.clone() }
});

pub struct DeactivateTaxConfigurationHandler {
    repo: Arc<dyn TaxConfigRepository>,
}

#[async_trait]
impl CommandHandler<DeactivateTaxConfiguration> for DeactivateTaxConfigurationHandler {
    async fn handle(&self, cmd: DeactivateTaxConfiguration) -> AppResult<TaxConfiguration> {
        let mut tc = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("TaxConfig", cmd.id.to_string()))?;
        tc.deactivate()?;
        self.repo.save(&tc).await?;
        Ok(tc)
    }
}

register_command_handler!(DeactivateTaxConfiguration, AppDeps, |d: &AppDeps| {
    DeactivateTaxConfigurationHandler { repo: d.tax_repo.clone() }
});

pub struct DeleteTaxConfigurationHandler {
    repo: Arc<dyn TaxConfigRepository>,
}

#[async_trait]
impl CommandHandler<DeleteTaxConfiguration> for DeleteTaxConfigurationHandler {
    async fn handle(&self, cmd: DeleteTaxConfiguration) -> AppResult<()> {
        self.repo.delete(cmd.id).await
    }
}

register_command_handler!(DeleteTaxConfiguration, AppDeps, |d: &AppDeps| {
    DeleteTaxConfigurationHandler { repo: d.tax_repo.clone() }
});

pub struct GetTaxConfigHandler {
    repo: Arc<dyn TaxConfigRepository>,
}

#[async_trait]
impl QueryHandler<GetTaxConfig> for GetTaxConfigHandler {
    async fn handle(&self, q: GetTaxConfig) -> AppResult<Option<TaxConfiguration>> {
        self.repo.find_by_id(q.id).await
    }
}

register_query_handler!(GetTaxConfig, AppDeps, |d: &AppDeps| {
    GetTaxConfigHandler { repo: d.tax_repo.clone() }
});

pub struct ListTaxConfigsHandler {
    repo: Arc<dyn TaxConfigRepository>,
}

#[async_trait]
impl QueryHandler<ListTaxConfigs> for ListTaxConfigsHandler {
    async fn handle(&self, q: ListTaxConfigs) -> AppResult<Vec<TaxConfiguration>> {
        self.repo.get_filtered(q.location_id, q.tax_type.as_deref(), q.active_only).await
    }
}

register_query_handler!(ListTaxConfigs, AppDeps, |d: &AppDeps| {
    ListTaxConfigsHandler { repo: d.tax_repo.clone() }
});

pub struct GetApplicableTaxConfigsHandler {
    repo: Arc<dyn TaxConfigRepository>,
}

#[async_trait]
impl QueryHandler<GetApplicableTaxConfigs> for GetApplicableTaxConfigsHandler {
    async fn handle(&self, q: GetApplicableTaxConfigs) -> AppResult<Vec<TaxConfiguration>> {
        self.repo.get_applicable(q.location_id, q.category_id).await
    }
}

register_query_handler!(GetApplicableTaxConfigs, AppDeps, |d: &AppDeps| {
    GetApplicableTaxConfigsHandler { repo: d.tax_repo.clone() }
});

pub struct CalculateTaxHandler {
    repo: Arc<dyn TaxConfigRepository>,
}

#[async_trait]
impl QueryHandler<CalculateTax> for CalculateTaxHandler {
    async fn handle(&self, q: CalculateTax) -> AppResult<(f64, f64, Vec<(String, String, String, f64, f64)>)> {
        let configs = self.repo.get_applicable(q.location_id, q.category_id).await?;
        let mut tax_amount = 0.0_f64;
        let mut applied = Vec::new();
        for tc in &configs {
            let amt = q.amount * tc.tax_rate / 100.0;
            tax_amount += amt;
            applied.push((tc.name.clone(), tc.code.clone(), tc.tax_type.clone(), tc.tax_rate, amt));
        }
        Ok((tax_amount, q.amount + tax_amount, applied))
    }
}

register_query_handler!(CalculateTax, AppDeps, |d: &AppDeps| {
    CalculateTaxHandler { repo: d.tax_repo.clone() }
});
