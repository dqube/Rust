use ddd_application::{register_command_handler, register_query_handler};
use ddd_shared_kernel::AppError;

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::queries::*;
use crate::domain::entities::Brand;

register_command_handler!(CreateBrand, AppDeps, |d: &AppDeps| {
    let repo = d.brand_repo.clone();
    move |cmd: CreateBrand| {
        let repo = repo.clone();
        async move {
            let brand = Brand::create(cmd.name, cmd.description, cmd.website)?;
            repo.save(&brand).await?;
            Ok(brand)
        }
    }
});

register_command_handler!(UpdateBrand, AppDeps, |d: &AppDeps| {
    let repo = d.brand_repo.clone();
    move |cmd: UpdateBrand| {
        let repo = repo.clone();
        async move {
            let mut b = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Brand {} not found", cmd.id)))?;
            b.update(cmd.name, cmd.description, cmd.website)?;
            repo.save(&b).await?;
            Ok(b)
        }
    }
});

register_command_handler!(ActivateBrand, AppDeps, |d: &AppDeps| {
    let repo = d.brand_repo.clone();
    move |cmd: ActivateBrand| {
        let repo = repo.clone();
        async move {
            let mut b = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Brand {} not found", cmd.id)))?;
            b.activate()?;
            repo.save(&b).await?;
            Ok(b)
        }
    }
});

register_command_handler!(DeactivateBrand, AppDeps, |d: &AppDeps| {
    let repo = d.brand_repo.clone();
    move |cmd: DeactivateBrand| {
        let repo = repo.clone();
        async move {
            let mut b = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Brand {} not found", cmd.id)))?;
            b.deactivate()?;
            repo.save(&b).await?;
            Ok(b)
        }
    }
});

register_query_handler!(GetBrand, AppDeps, |d: &AppDeps| {
    let repo = d.brand_repo.clone();
    move |q: GetBrand| {
        let repo = repo.clone();
        async move { repo.find_by_id(q.id).await }
    }
});

register_query_handler!(ListBrands, AppDeps, |d: &AppDeps| {
    let repo = d.brand_repo.clone();
    move |q: ListBrands| {
        let repo = repo.clone();
        async move {
            repo.get_paged(q.search.as_deref(), q.active_only, &q.req).await
        }
    }
});
