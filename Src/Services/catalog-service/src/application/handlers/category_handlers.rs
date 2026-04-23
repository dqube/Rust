use ddd_application::{register_command_handler, register_query_handler};
use ddd_shared_kernel::AppError;

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::queries::*;
use crate::domain::entities::ProductCategory;

register_command_handler!(CreateCategory, AppDeps, |d: &AppDeps| {
    let repo    = d.category_repo.clone();
    move |cmd: CreateCategory| {
        let repo    = repo.clone();
        async move {
            let mut cat = ProductCategory::create(cmd.name, cmd.description, cmd.parent_category_id)?;
            let new_id = repo.insert(&cat).await?;
            cat.id = crate::domain::ids::CategoryId(new_id);
            Ok(cat)
        }
    }
});

register_command_handler!(UpdateCategory, AppDeps, |d: &AppDeps| {
    let repo = d.category_repo.clone();
    move |cmd: UpdateCategory| {
        let repo = repo.clone();
        async move {
            let mut c = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Category {} not found", cmd.id)))?;
            c.update(cmd.name, cmd.description, cmd.parent_category_id)?;
            repo.save(&c).await?;
            Ok(c)
        }
    }
});

register_command_handler!(DeleteCategory, AppDeps, |d: &AppDeps| {
    let repo = d.category_repo.clone();
    move |cmd: DeleteCategory| {
        let repo = repo.clone();
        async move { repo.delete(cmd.id).await }
    }
});

register_command_handler!(RequestCategoryImageUploadUrl, AppDeps, |d: &AppDeps| {
    let repo    = d.category_repo.clone();
    let storage = d.blob_storage.clone();
    let bucket  = d.blob_bucket.clone();
    let ttl     = d.presign_ttl_secs;
    move |cmd: RequestCategoryImageUploadUrl| {
        let repo    = repo.clone();
        let storage = storage.clone();
        let bucket  = bucket.clone();
        async move {
            repo.find_by_id(cmd.category_id).await?
                .ok_or_else(|| AppError::not_found(format!("Category {} not found", cmd.category_id)))?;
            let ext = cmd.file_name.rsplit('.').next().unwrap_or("bin");
            let object_name = format!("catalog/categories/{}/{}.{}", cmd.category_id.0, uuid::Uuid::new_v4(), ext);
            let presigned = storage
                .presigned_put(&bucket, &object_name, &cmd.content_type, std::time::Duration::from_secs(ttl))
                .await
                .map_err(|e| AppError::internal(e.to_string()))?;
            Ok((presigned.url, object_name, presigned.expires_at.to_rfc3339()))
        }
    }
});

register_command_handler!(ConfirmCategoryImageUpload, AppDeps, |d: &AppDeps| {
    let repo = d.category_repo.clone();
    move |cmd: ConfirmCategoryImageUpload| {
        let repo = repo.clone();
        async move {
            let mut c = repo.find_by_id(cmd.category_id).await?
                .ok_or_else(|| AppError::not_found(format!("Category {} not found", cmd.category_id)))?;
            c.set_image_url(cmd.public_url);
            repo.save(&c).await?;
            Ok(c)
        }
    }
});

register_query_handler!(GetCategory, AppDeps, |d: &AppDeps| {
    let repo = d.category_repo.clone();
    move |q: GetCategory| {
        let repo = repo.clone();
        async move { repo.find_by_id(q.id).await }
    }
});

register_query_handler!(ListCategories, AppDeps, |d: &AppDeps| {
    let repo = d.category_repo.clone();
    move |q: ListCategories| {
        let repo = repo.clone();
        async move { repo.get_all(q.parent_id).await }
    }
});
