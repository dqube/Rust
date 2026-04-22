use ddd_application::{register_command_handler, register_query_handler};
use ddd_shared_kernel::AppError;

use crate::application::commands::{CreateDesignation, UpdateDesignation};
use crate::application::deps::AppDeps;
use crate::application::queries::{GetDesignation, ListDesignations};
use crate::domain::entities::Designation;

register_command_handler!(CreateDesignation, AppDeps, |d: &AppDeps| {
    let repo = d.designation_repo.clone();
    move |cmd: CreateDesignation| {
        let repo = repo.clone();
        async move {
            if repo.name_exists(&cmd.designation_name).await? {
                return Err(AppError::conflict(format!("Designation '{}' already exists", cmd.designation_name)));
            }
            let desig = Designation::create(cmd.designation_name, cmd.level);
            repo.save(&desig).await?;
            Ok(desig)
        }
    }
});

register_command_handler!(UpdateDesignation, AppDeps, |d: &AppDeps| {
    let repo = d.designation_repo.clone();
    move |cmd: UpdateDesignation| {
        let repo = repo.clone();
        async move {
            let mut desig = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Designation {} not found", cmd.id)))?;
            desig.designation_name = cmd.designation_name;
            desig.level = cmd.level;
            desig.updated_at = Some(chrono::Utc::now());
            repo.save(&desig).await?;
            Ok(desig)
        }
    }
});

register_query_handler!(GetDesignation, AppDeps, |d: &AppDeps| {
    let repo = d.designation_repo.clone();
    move |q: GetDesignation| {
        let repo = repo.clone();
        async move { repo.find_by_id(q.id).await }
    }
});

register_query_handler!(ListDesignations, AppDeps, |d: &AppDeps| {
    let repo = d.designation_repo.clone();
    move |_q: ListDesignations| {
        let repo = repo.clone();
        async move { repo.get_all().await }
    }
});
