use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use ddd_application::{CommandHandler, QueryHandler, register_command_handler, register_query_handler};
use ddd_shared_kernel::{AppError, AppResult, BlobStorage};

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::queries::*;
use crate::domain::entities::ProductCategory;
use crate::domain::repositories::CategoryRepository;

pub struct CreateCategoryHandler {
    repo: Arc<dyn CategoryRepository>,
}

#[async_trait]
impl CommandHandler<CreateCategory> for CreateCategoryHandler {
    async fn handle(&self, cmd: CreateCategory) -> AppResult<ProductCategory> {
        let mut cat = ProductCategory::create(cmd.name, cmd.description, cmd.parent_category_id)?;
        let new_id = self.repo.insert(&cat).await?;
        cat.id = crate::domain::ids::CategoryId(new_id);
        Ok(cat)
    }
}

register_command_handler!(CreateCategory, AppDeps, |d: &AppDeps| {
    CreateCategoryHandler { repo: d.category_repo.clone() }
});

pub struct UpdateCategoryHandler {
    repo: Arc<dyn CategoryRepository>,
}

#[async_trait]
impl CommandHandler<UpdateCategory> for UpdateCategoryHandler {
    async fn handle(&self, cmd: UpdateCategory) -> AppResult<ProductCategory> {
        let mut c = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Category", cmd.id.to_string()))?;
        c.update(cmd.name, cmd.description, cmd.parent_category_id)?;
        self.repo.save(&c).await?;
        Ok(c)
    }
}

register_command_handler!(UpdateCategory, AppDeps, |d: &AppDeps| {
    UpdateCategoryHandler { repo: d.category_repo.clone() }
});

pub struct DeleteCategoryHandler {
    repo: Arc<dyn CategoryRepository>,
}

#[async_trait]
impl CommandHandler<DeleteCategory> for DeleteCategoryHandler {
    async fn handle(&self, cmd: DeleteCategory) -> AppResult<()> {
        self.repo.delete(cmd.id).await
    }
}

register_command_handler!(DeleteCategory, AppDeps, |d: &AppDeps| {
    DeleteCategoryHandler { repo: d.category_repo.clone() }
});

pub struct RequestCategoryImageUploadUrlHandler {
    repo:             Arc<dyn CategoryRepository>,
    blob_storage:     Arc<dyn BlobStorage>,
    blob_bucket:      String,
    presign_ttl_secs: u64,
}

#[async_trait]
impl CommandHandler<RequestCategoryImageUploadUrl> for RequestCategoryImageUploadUrlHandler {
    async fn handle(&self, cmd: RequestCategoryImageUploadUrl) -> AppResult<(String, String, String)> {
        self.repo.find_by_id(cmd.category_id).await?
            .ok_or_else(|| AppError::not_found("Category", cmd.category_id.to_string()))?;
        let ext = cmd.file_name.rsplit('.').next().unwrap_or("bin");
        let object_name = format!("catalog/categories/{}/{}.{}", cmd.category_id.0, uuid::Uuid::new_v4(), ext);
        let presigned = self.blob_storage
            .presigned_put(&self.blob_bucket, &object_name, &cmd.content_type, Duration::from_secs(self.presign_ttl_secs))
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        Ok((presigned.url, object_name, presigned.expires_at.to_rfc3339()))
    }
}

register_command_handler!(RequestCategoryImageUploadUrl, AppDeps, |d: &AppDeps| {
    RequestCategoryImageUploadUrlHandler {
        repo:             d.category_repo.clone(),
        blob_storage:     d.blob_storage.clone(),
        blob_bucket:      d.blob_bucket.clone(),
        presign_ttl_secs: d.presign_ttl_secs,
    }
});

pub struct ConfirmCategoryImageUploadHandler {
    repo: Arc<dyn CategoryRepository>,
}

#[async_trait]
impl CommandHandler<ConfirmCategoryImageUpload> for ConfirmCategoryImageUploadHandler {
    async fn handle(&self, cmd: ConfirmCategoryImageUpload) -> AppResult<ProductCategory> {
        let mut c = self.repo.find_by_id(cmd.category_id).await?
            .ok_or_else(|| AppError::not_found("Category", cmd.category_id.to_string()))?;
        c.set_image_url(cmd.public_url);
        self.repo.save(&c).await?;
        Ok(c)
    }
}

register_command_handler!(ConfirmCategoryImageUpload, AppDeps, |d: &AppDeps| {
    ConfirmCategoryImageUploadHandler { repo: d.category_repo.clone() }
});

pub struct GetCategoryHandler {
    repo: Arc<dyn CategoryRepository>,
}

#[async_trait]
impl QueryHandler<GetCategory> for GetCategoryHandler {
    async fn handle(&self, q: GetCategory) -> AppResult<Option<ProductCategory>> {
        self.repo.find_by_id(q.id).await
    }
}

register_query_handler!(GetCategory, AppDeps, |d: &AppDeps| {
    GetCategoryHandler { repo: d.category_repo.clone() }
});

pub struct ListCategoriesHandler {
    repo: Arc<dyn CategoryRepository>,
}

#[async_trait]
impl QueryHandler<ListCategories> for ListCategoriesHandler {
    async fn handle(&self, q: ListCategories) -> AppResult<Vec<ProductCategory>> {
        self.repo.get_all(q.parent_id).await
    }
}

register_query_handler!(ListCategories, AppDeps, |d: &AppDeps| {
    ListCategoriesHandler { repo: d.category_repo.clone() }
});
