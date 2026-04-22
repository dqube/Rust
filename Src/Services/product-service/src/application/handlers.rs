use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use ddd_application::{
    register_command_handler, register_query_handler, CommandHandler, QueryHandler,
};
use ddd_shared_kernel::{AggregateRoot, AppError, AppResult, BlobStorage, Page, PageRequest};

use super::commands::{
    ConfirmImageUpload, CreateProduct, DeactivateProduct, RequestImageUploadUrl, UpdateStock,
};
use super::deps::AppDeps;
use super::queries::{GetProduct, ListProducts};
use crate::domain::aggregate::Product;
use crate::domain::events::ProductId;
#[allow(unused_imports)]
use crate::domain::repository::ProductRepository;

pub struct CreateProductHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<CreateProduct> for CreateProductHandler {
    async fn handle(&self, cmd: CreateProduct) -> AppResult<ProductId> {
        let p = Product::create(cmd.sku, cmd.name, cmd.description, cmd.price_cents, cmd.stock)?;
        self.repo.save(&p).await?;
        Ok(*p.id())
    }
}

register_command_handler!(CreateProduct, AppDeps, |deps: &AppDeps| {
    CreateProductHandler {
        repo: deps.product_repo.clone(),
    }
});

pub struct UpdateStockHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<UpdateStock> for UpdateStockHandler {
    async fn handle(&self, cmd: UpdateStock) -> AppResult<()> {
        let mut p = self
            .repo
            .find_by_id(cmd.product_id)
            .await?
            .ok_or_else(|| AppError::not_found("Product", cmd.product_id.to_string()))?;
        p.update_stock(cmd.stock)?;
        self.repo.update(&p).await
    }
}

register_command_handler!(UpdateStock, AppDeps, |deps: &AppDeps| {
    UpdateStockHandler {
        repo: deps.product_repo.clone(),
    }
});

pub struct DeactivateProductHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<DeactivateProduct> for DeactivateProductHandler {
    async fn handle(&self, cmd: DeactivateProduct) -> AppResult<()> {
        let mut p = self
            .repo
            .find_by_id(cmd.product_id)
            .await?
            .ok_or_else(|| AppError::not_found("Product", cmd.product_id.to_string()))?;
        p.deactivate()?;
        self.repo.update(&p).await
    }
}

register_command_handler!(DeactivateProduct, AppDeps, |deps: &AppDeps| {
    DeactivateProductHandler {
        repo: deps.product_repo.clone(),
    }
});

pub struct GetProductHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl QueryHandler<GetProduct> for GetProductHandler {
    async fn handle(&self, q: GetProduct) -> AppResult<Option<Product>> {
        self.repo.find_by_id(q.product_id).await
    }
}

register_query_handler!(GetProduct, AppDeps, |deps: &AppDeps| {
    GetProductHandler {
        repo: deps.product_repo.clone(),
    }
});

pub struct ListProductsHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl QueryHandler<ListProducts> for ListProductsHandler {
    async fn handle(&self, q: ListProducts) -> AppResult<Page<Product>> {
        self.repo.find_all_paginated(PageRequest::new(q.page, q.per_page)).await
    }
}

register_query_handler!(ListProducts, AppDeps, |deps: &AppDeps| {
    ListProductsHandler {
        repo: deps.product_repo.clone(),
    }
});

pub struct RequestImageUploadUrlHandler {
    repo: Arc<dyn ProductRepository>,
    storage: Arc<dyn BlobStorage>,
    bucket: String,
    ttl: Duration,
}

#[async_trait]
impl CommandHandler<RequestImageUploadUrl> for RequestImageUploadUrlHandler {
    async fn handle(&self, cmd: RequestImageUploadUrl) -> AppResult<(String, u32)> {
        // Verify the product exists before issuing a URL.
        self.repo
            .find_by_id(cmd.product_id)
            .await?
            .ok_or_else(|| AppError::not_found("Product", cmd.product_id.to_string()))?;

        let object_key = format!("products/{}/{}", cmd.product_id, cmd.filename);
        let presigned = self
            .storage
            .presigned_put(&self.bucket, &object_key, &cmd.content_type, self.ttl)
            .await?;
        // u32 cap is intentional — callers don't need sub-second precision and
        // a TTL longer than ~136 years would be a configuration bug.
        let expires_in_secs = u32::try_from(self.ttl.as_secs()).unwrap_or(u32::MAX);
        Ok((presigned.url, expires_in_secs))
    }
}

register_command_handler!(RequestImageUploadUrl, AppDeps, |deps: &AppDeps| {
    RequestImageUploadUrlHandler {
        repo: deps.product_repo.clone(),
        storage: deps.storage.clone(),
        bucket: deps.image_bucket.clone(),
        ttl: deps.presign_ttl,
    }
});

pub struct ConfirmImageUploadHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<ConfirmImageUpload> for ConfirmImageUploadHandler {
    async fn handle(&self, cmd: ConfirmImageUpload) -> AppResult<()> {
        let mut p = self
            .repo
            .find_by_id(cmd.product_id)
            .await?
            .ok_or_else(|| AppError::not_found("Product", cmd.product_id.to_string()))?;
        p.update_image(cmd.image_url)?;
        self.repo.update(&p).await
    }
}

register_command_handler!(ConfirmImageUpload, AppDeps, |deps: &AppDeps| {
    ConfirmImageUploadHandler {
        repo: deps.product_repo.clone(),
    }
});
