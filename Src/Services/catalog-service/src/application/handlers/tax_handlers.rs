use ddd_application::{register_command_handler, register_query_handler};
use ddd_shared_kernel::AppError;

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::queries::*;
use crate::domain::entities::TaxConfiguration;

register_command_handler!(CreateTaxConfiguration, AppDeps, |d: &AppDeps| {
    let repo = d.tax_repo.clone();
    move |cmd: CreateTaxConfiguration| {
        let repo = repo.clone();
        async move {
            let tc = TaxConfiguration::create(
                cmd.name, cmd.code, cmd.tax_type, cmd.tax_rate,
                cmd.location_id, cmd.category_id, cmd.effective_date, cmd.expiry_date,
            )?;
            repo.save(&tc).await?;
            Ok(tc)
        }
    }
});

register_command_handler!(UpdateTaxConfiguration, AppDeps, |d: &AppDeps| {
    let repo = d.tax_repo.clone();
    move |cmd: UpdateTaxConfiguration| {
        let repo = repo.clone();
        async move {
            let mut tc = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("TaxConfig {} not found", cmd.id)))?;
            tc.update(cmd.name, cmd.code, cmd.tax_type, cmd.tax_rate, cmd.effective_date, cmd.expiry_date)?;
            repo.save(&tc).await?;
            Ok(tc)
        }
    }
});

register_command_handler!(ActivateTaxConfiguration, AppDeps, |d: &AppDeps| {
    let repo = d.tax_repo.clone();
    move |cmd: ActivateTaxConfiguration| {
        let repo = repo.clone();
        async move {
            let mut tc = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("TaxConfig {} not found", cmd.id)))?;
            tc.activate()?;
            repo.save(&tc).await?;
            Ok(tc)
        }
    }
});

register_command_handler!(DeactivateTaxConfiguration, AppDeps, |d: &AppDeps| {
    let repo = d.tax_repo.clone();
    move |cmd: DeactivateTaxConfiguration| {
        let repo = repo.clone();
        async move {
            let mut tc = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("TaxConfig {} not found", cmd.id)))?;
            tc.deactivate()?;
            repo.save(&tc).await?;
            Ok(tc)
        }
    }
});

register_command_handler!(DeleteTaxConfiguration, AppDeps, |d: &AppDeps| {
    let repo = d.tax_repo.clone();
    move |cmd: DeleteTaxConfiguration| {
        let repo = repo.clone();
        async move { repo.delete(cmd.id).await }
    }
});

register_query_handler!(GetTaxConfig, AppDeps, |d: &AppDeps| {
    let repo = d.tax_repo.clone();
    move |q: GetTaxConfig| {
        let repo = repo.clone();
        async move { repo.find_by_id(q.id).await }
    }
});

register_query_handler!(ListTaxConfigs, AppDeps, |d: &AppDeps| {
    let repo = d.tax_repo.clone();
    move |q: ListTaxConfigs| {
        let repo = repo.clone();
        async move {
            repo.get_filtered(q.location_id, q.tax_type.as_deref(), q.active_only).await
        }
    }
});

register_query_handler!(GetApplicableTaxConfigs, AppDeps, |d: &AppDeps| {
    let repo = d.tax_repo.clone();
    move |q: GetApplicableTaxConfigs| {
        let repo = repo.clone();
        async move { repo.get_applicable(q.location_id, q.category_id).await }
    }
});

register_query_handler!(CalculateTax, AppDeps, |d: &AppDeps| {
    let repo = d.tax_repo.clone();
    move |q: CalculateTax| {
        let repo = repo.clone();
        async move {
            let configs = repo.get_applicable(q.location_id, q.category_id).await?;
            let mut tax_amount = 0.0_f64;
            let mut applied = Vec::new();
            for tc in &configs {
                let amt = q.amount * tc.tax_rate / 100.0;
                tax_amount += amt;
                applied.push((
                    tc.name.clone(),
                    tc.code.clone(),
                    tc.tax_type.clone(),
                    tc.tax_rate,
                    amt,
                ));
            }
            Ok((tax_amount, q.amount + tax_amount, applied))
        }
    }
});
