use std::sync::Arc;

use async_trait::async_trait;
use ddd_application::{CommandHandler, QueryHandler, register_command_handler, register_query_handler};
use ddd_shared_kernel::{AppError, AppResult, Page};

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::queries::*;
use crate::domain::entities::Brand;
use crate::domain::repositories::BrandRepository;

pub struct CreateBrandHandler {
    repo: Arc<dyn BrandRepository>,
}

#[async_trait]
impl CommandHandler<CreateBrand> for CreateBrandHandler {
    async fn handle(&self, cmd: CreateBrand) -> AppResult<Brand> {
        let brand = Brand::create(cmd.name, cmd.description, cmd.website)?;
        self.repo.save(&brand).await?;
        Ok(brand)
    }
}

register_command_handler!(CreateBrand, AppDeps, |d: &AppDeps| {
    CreateBrandHandler { repo: d.brand_repo.clone() }
});

pub struct UpdateBrandHandler {
    repo: Arc<dyn BrandRepository>,
}

#[async_trait]
impl CommandHandler<UpdateBrand> for UpdateBrandHandler {
    async fn handle(&self, cmd: UpdateBrand) -> AppResult<Brand> {
        let mut b = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Brand", cmd.id.to_string()))?;
        b.update(cmd.name, cmd.description, cmd.website)?;
        self.repo.save(&b).await?;
        Ok(b)
    }
}

register_command_handler!(UpdateBrand, AppDeps, |d: &AppDeps| {
    UpdateBrandHandler { repo: d.brand_repo.clone() }
});

pub struct ActivateBrandHandler {
    repo: Arc<dyn BrandRepository>,
}

#[async_trait]
impl CommandHandler<ActivateBrand> for ActivateBrandHandler {
    async fn handle(&self, cmd: ActivateBrand) -> AppResult<Brand> {
        let mut b = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Brand", cmd.id.to_string()))?;
        b.activate()?;
        self.repo.save(&b).await?;
        Ok(b)
    }
}

register_command_handler!(ActivateBrand, AppDeps, |d: &AppDeps| {
    ActivateBrandHandler { repo: d.brand_repo.clone() }
});

pub struct DeactivateBrandHandler {
    repo: Arc<dyn BrandRepository>,
}

#[async_trait]
impl CommandHandler<DeactivateBrand> for DeactivateBrandHandler {
    async fn handle(&self, cmd: DeactivateBrand) -> AppResult<Brand> {
        let mut b = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Brand", cmd.id.to_string()))?;
        b.deactivate()?;
        self.repo.save(&b).await?;
        Ok(b)
    }
}

register_command_handler!(DeactivateBrand, AppDeps, |d: &AppDeps| {
    DeactivateBrandHandler { repo: d.brand_repo.clone() }
});

pub struct GetBrandHandler {
    repo: Arc<dyn BrandRepository>,
}

#[async_trait]
impl QueryHandler<GetBrand> for GetBrandHandler {
    async fn handle(&self, q: GetBrand) -> AppResult<Option<Brand>> {
        self.repo.find_by_id(q.id).await
    }
}

register_query_handler!(GetBrand, AppDeps, |d: &AppDeps| {
    GetBrandHandler { repo: d.brand_repo.clone() }
});

pub struct ListBrandsHandler {
    repo: Arc<dyn BrandRepository>,
}

#[async_trait]
impl QueryHandler<ListBrands> for ListBrandsHandler {
    async fn handle(&self, q: ListBrands) -> AppResult<Page<Brand>> {
        self.repo.get_paged(q.search.as_deref(), q.active_only, &q.req).await
    }
}

register_query_handler!(ListBrands, AppDeps, |d: &AppDeps| {
    ListBrandsHandler { repo: d.brand_repo.clone() }
});
