use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use ddd_application::{CommandHandler, QueryHandler, register_command_handler, register_query_handler};
use ddd_shared_kernel::{AppError, AppResult, BlobStorage, Page};

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::queries::*;
use crate::domain::entities::{Product, ProductImage, ProductVariant};
use crate::domain::repositories::ProductRepository;

// ── CreateProduct ─────────────────────────────────────────────────────────────

pub struct CreateProductHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<CreateProduct> for CreateProductHandler {
    async fn handle(&self, cmd: CreateProduct) -> AppResult<Product> {
        let product = Product::create(
            cmd.sku, cmd.name, cmd.category_id,
            cmd.base_price, cmd.cost_price, cmd.description, cmd.is_taxable,
        )?;
        self.repo.save(&product).await?;
        Ok(product)
    }
}

register_command_handler!(CreateProduct, AppDeps, |d: &AppDeps| {
    CreateProductHandler { repo: d.product_repo.clone() }
});

// ── UpdateProduct ─────────────────────────────────────────────────────────────

pub struct UpdateProductHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<UpdateProduct> for UpdateProductHandler {
    async fn handle(&self, cmd: UpdateProduct) -> AppResult<Product> {
        let mut p = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Product", cmd.id.to_string()))?;
        p.update_basic_info(cmd.name, cmd.category_id, cmd.base_price, cmd.cost_price, cmd.is_taxable, cmd.description)?;
        self.repo.save(&p).await?;
        Ok(p)
    }
}

register_command_handler!(UpdateProduct, AppDeps, |d: &AppDeps| {
    UpdateProductHandler { repo: d.product_repo.clone() }
});

// ── DiscontinueProduct ────────────────────────────────────────────────────────

pub struct DiscontinueProductHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<DiscontinueProduct> for DiscontinueProductHandler {
    async fn handle(&self, cmd: DiscontinueProduct) -> AppResult<Product> {
        let mut p = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Product", cmd.id.to_string()))?;
        p.discontinue()?;
        self.repo.save(&p).await?;
        Ok(p)
    }
}

register_command_handler!(DiscontinueProduct, AppDeps, |d: &AppDeps| {
    DiscontinueProductHandler { repo: d.product_repo.clone() }
});

// ── ReactivateProduct ─────────────────────────────────────────────────────────

pub struct ReactivateProductHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<ReactivateProduct> for ReactivateProductHandler {
    async fn handle(&self, cmd: ReactivateProduct) -> AppResult<Product> {
        let mut p = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Product", cmd.id.to_string()))?;
        p.reactivate()?;
        self.repo.save(&p).await?;
        Ok(p)
    }
}

register_command_handler!(ReactivateProduct, AppDeps, |d: &AppDeps| {
    ReactivateProductHandler { repo: d.product_repo.clone() }
});

// ── UpdateProductPricing ──────────────────────────────────────────────────────

pub struct UpdateProductPricingHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<UpdateProductPricing> for UpdateProductPricingHandler {
    async fn handle(&self, cmd: UpdateProductPricing) -> AppResult<Product> {
        let mut p = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Product", cmd.id.to_string()))?;
        p.update_pricing(cmd.price);
        self.repo.save(&p).await?;
        Ok(p)
    }
}

register_command_handler!(UpdateProductPricing, AppDeps, |d: &AppDeps| {
    UpdateProductPricingHandler { repo: d.product_repo.clone() }
});

// ── AssignProductBrand ────────────────────────────────────────────────────────

pub struct AssignProductBrandHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<AssignProductBrand> for AssignProductBrandHandler {
    async fn handle(&self, cmd: AssignProductBrand) -> AppResult<Product> {
        let mut p = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Product", cmd.id.to_string()))?;
        p.assign_brand(cmd.brand_id);
        self.repo.save(&p).await?;
        Ok(p)
    }
}

register_command_handler!(AssignProductBrand, AppDeps, |d: &AppDeps| {
    AssignProductBrandHandler { repo: d.product_repo.clone() }
});

// ── SetProductDimensions ──────────────────────────────────────────────────────

pub struct SetProductDimensionsHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<SetProductDimensions> for SetProductDimensionsHandler {
    async fn handle(&self, cmd: SetProductDimensions) -> AppResult<Product> {
        let mut p = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Product", cmd.id.to_string()))?;
        p.set_dimensions(cmd.weight_grams, cmd.width_cm, cmd.height_cm, cmd.depth_cm);
        self.repo.save(&p).await?;
        Ok(p)
    }
}

register_command_handler!(SetProductDimensions, AppDeps, |d: &AppDeps| {
    SetProductDimensionsHandler { repo: d.product_repo.clone() }
});

// ── SetProductSpecifications ──────────────────────────────────────────────────

pub struct SetProductSpecificationsHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<SetProductSpecifications> for SetProductSpecificationsHandler {
    async fn handle(&self, cmd: SetProductSpecifications) -> AppResult<Product> {
        let mut p = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Product", cmd.id.to_string()))?;
        p.set_specifications(cmd.specs);
        self.repo.save(&p).await?;
        Ok(p)
    }
}

register_command_handler!(SetProductSpecifications, AppDeps, |d: &AppDeps| {
    SetProductSpecificationsHandler { repo: d.product_repo.clone() }
});

// ── SetProductTags ────────────────────────────────────────────────────────────

pub struct SetProductTagsHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<SetProductTags> for SetProductTagsHandler {
    async fn handle(&self, cmd: SetProductTags) -> AppResult<Product> {
        let mut p = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Product", cmd.id.to_string()))?;
        p.set_tags(cmd.tags);
        self.repo.save(&p).await?;
        Ok(p)
    }
}

register_command_handler!(SetProductTags, AppDeps, |d: &AppDeps| {
    SetProductTagsHandler { repo: d.product_repo.clone() }
});

// ── SetProductTaxConfigurations ───────────────────────────────────────────────

pub struct SetProductTaxConfigurationsHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<SetProductTaxConfigurations> for SetProductTaxConfigurationsHandler {
    async fn handle(&self, cmd: SetProductTaxConfigurations) -> AppResult<Product> {
        let mut p = self.repo.find_by_id(cmd.id).await?
            .ok_or_else(|| AppError::not_found("Product", cmd.id.to_string()))?;
        p.set_tax_configurations(cmd.tax_config_ids);
        self.repo.save(&p).await?;
        Ok(p)
    }
}

register_command_handler!(SetProductTaxConfigurations, AppDeps, |d: &AppDeps| {
    SetProductTaxConfigurationsHandler { repo: d.product_repo.clone() }
});

// ── AddProductVariant ─────────────────────────────────────────────────────────

pub struct AddProductVariantHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<AddProductVariant> for AddProductVariantHandler {
    async fn handle(&self, cmd: AddProductVariant) -> AppResult<Product> {
        let mut p = self.repo.find_by_id(cmd.product_id).await?
            .ok_or_else(|| AppError::not_found("Product", cmd.product_id.to_string()))?;
        let variant = ProductVariant::create(
            cmd.product_id, cmd.sku, &cmd.attributes_json,
            cmd.price_override, cmd.description, cmd.cost_price_override,
            cmd.barcode, cmd.barcode_type, cmd.weight_grams, cmd.width_cm, cmd.height_cm, cmd.depth_cm,
        );
        p.add_variant(variant);
        self.repo.save(&p).await?;
        Ok(p)
    }
}

register_command_handler!(AddProductVariant, AppDeps, |d: &AppDeps| {
    AddProductVariantHandler { repo: d.product_repo.clone() }
});

// ── UpdateProductVariant ──────────────────────────────────────────────────────

pub struct UpdateProductVariantHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<UpdateProductVariant> for UpdateProductVariantHandler {
    async fn handle(&self, cmd: UpdateProductVariant) -> AppResult<Product> {
        let mut p = self.repo.find_by_id(cmd.product_id).await?
            .ok_or_else(|| AppError::not_found("Product", cmd.product_id.to_string()))?;
        p.update_variant(
            cmd.variant_id, cmd.sku, &cmd.attributes_json,
            cmd.price_override, cmd.description, cmd.is_active,
            cmd.cost_price_override, cmd.barcode, cmd.barcode_type,
            cmd.weight_grams, cmd.width_cm, cmd.height_cm, cmd.depth_cm,
        )?;
        self.repo.save(&p).await?;
        Ok(p)
    }
}

register_command_handler!(UpdateProductVariant, AppDeps, |d: &AppDeps| {
    UpdateProductVariantHandler { repo: d.product_repo.clone() }
});

// ── RemoveProductVariant ──────────────────────────────────────────────────────

pub struct RemoveProductVariantHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<RemoveProductVariant> for RemoveProductVariantHandler {
    async fn handle(&self, cmd: RemoveProductVariant) -> AppResult<Product> {
        let mut p = self.repo.find_by_id(cmd.product_id).await?
            .ok_or_else(|| AppError::not_found("Product", cmd.product_id.to_string()))?;
        p.remove_variant(cmd.variant_id)?;
        self.repo.save(&p).await?;
        Ok(p)
    }
}

register_command_handler!(RemoveProductVariant, AppDeps, |d: &AppDeps| {
    RemoveProductVariantHandler { repo: d.product_repo.clone() }
});

// ── SetDefaultVariant ─────────────────────────────────────────────────────────

pub struct SetDefaultVariantHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<SetDefaultVariant> for SetDefaultVariantHandler {
    async fn handle(&self, cmd: SetDefaultVariant) -> AppResult<Product> {
        let mut p = self.repo.find_by_id(cmd.product_id).await?
            .ok_or_else(|| AppError::not_found("Product", cmd.product_id.to_string()))?;
        p.set_default_variant(cmd.variant_id)?;
        self.repo.save(&p).await?;
        Ok(p)
    }
}

register_command_handler!(SetDefaultVariant, AppDeps, |d: &AppDeps| {
    SetDefaultVariantHandler { repo: d.product_repo.clone() }
});

// ── RequestProductImageUploadUrl ──────────────────────────────────────────────

pub struct RequestProductImageUploadUrlHandler {
    repo:             Arc<dyn ProductRepository>,
    blob_storage:     Arc<dyn BlobStorage>,
    blob_bucket:      String,
    presign_ttl_secs: u64,
}

#[async_trait]
impl CommandHandler<RequestProductImageUploadUrl> for RequestProductImageUploadUrlHandler {
    async fn handle(&self, cmd: RequestProductImageUploadUrl) -> AppResult<(String, String, String)> {
        self.repo.find_by_id(cmd.product_id).await?
            .ok_or_else(|| AppError::not_found("Product", cmd.product_id.to_string()))?;
        let ext = cmd.file_name.rsplit('.').next().unwrap_or("bin");
        let object_name = format!("catalog/products/{}/{}.{}", cmd.product_id, uuid::Uuid::new_v4(), ext);
        let presigned = self.blob_storage
            .presigned_put(&self.blob_bucket, &object_name, &cmd.content_type, Duration::from_secs(self.presign_ttl_secs))
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        Ok((presigned.url, object_name, presigned.expires_at.to_rfc3339()))
    }
}

register_command_handler!(RequestProductImageUploadUrl, AppDeps, |d: &AppDeps| {
    RequestProductImageUploadUrlHandler {
        repo:             d.product_repo.clone(),
        blob_storage:     d.blob_storage.clone(),
        blob_bucket:      d.blob_bucket.clone(),
        presign_ttl_secs: d.presign_ttl_secs,
    }
});

// ── ConfirmProductImageUpload ─────────────────────────────────────────────────

pub struct ConfirmProductImageUploadHandler {
    repo:             Arc<dyn ProductRepository>,
    blob_storage:     Arc<dyn BlobStorage>,
    blob_bucket:      String,
    presign_ttl_secs: u64,
}

#[async_trait]
impl CommandHandler<ConfirmProductImageUpload> for ConfirmProductImageUploadHandler {
    async fn handle(&self, cmd: ConfirmProductImageUpload) -> AppResult<Product> {
        let mut p = self.repo.find_by_id(cmd.product_id).await?
            .ok_or_else(|| AppError::not_found("Product", cmd.product_id.to_string()))?;
        let presigned = self.blob_storage
            .presigned_get(&self.blob_bucket, &cmd.object_name, Duration::from_secs(self.presign_ttl_secs))
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        let image = ProductImage::create(
            cmd.product_id, presigned.url, cmd.is_main, cmd.sort_order, cmd.alt_text,
        );
        p.add_image(image);
        self.repo.save(&p).await?;
        Ok(p)
    }
}

register_command_handler!(ConfirmProductImageUpload, AppDeps, |d: &AppDeps| {
    ConfirmProductImageUploadHandler {
        repo:             d.product_repo.clone(),
        blob_storage:     d.blob_storage.clone(),
        blob_bucket:      d.blob_bucket.clone(),
        presign_ttl_secs: d.presign_ttl_secs,
    }
});

// ── DeleteProductImage ────────────────────────────────────────────────────────

pub struct DeleteProductImageHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl CommandHandler<DeleteProductImage> for DeleteProductImageHandler {
    async fn handle(&self, cmd: DeleteProductImage) -> AppResult<Product> {
        let mut p = self.repo.find_by_id(cmd.product_id).await?
            .ok_or_else(|| AppError::not_found("Product", cmd.product_id.to_string()))?;
        p.remove_image(cmd.image_id)?;
        self.repo.save(&p).await?;
        Ok(p)
    }
}

register_command_handler!(DeleteProductImage, AppDeps, |d: &AppDeps| {
    DeleteProductImageHandler { repo: d.product_repo.clone() }
});

// ── GetProduct ────────────────────────────────────────────────────────────────

pub struct GetProductHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl QueryHandler<GetProduct> for GetProductHandler {
    async fn handle(&self, q: GetProduct) -> AppResult<Option<Product>> {
        self.repo.find_by_id(q.id).await
    }
}

register_query_handler!(GetProduct, AppDeps, |d: &AppDeps| {
    GetProductHandler { repo: d.product_repo.clone() }
});

// ── ListProducts ──────────────────────────────────────────────────────────────

pub struct ListProductsHandler {
    repo: Arc<dyn ProductRepository>,
}

#[async_trait]
impl QueryHandler<ListProducts> for ListProductsHandler {
    async fn handle(&self, q: ListProducts) -> AppResult<Page<Product>> {
        self.repo.get_paged(
            q.search.as_deref(), q.category_id, q.min_price, q.max_price,
            q.sort_by.as_deref(), q.sort_desc, &q.req,
        ).await
    }
}

register_query_handler!(ListProducts, AppDeps, |d: &AppDeps| {
    ListProductsHandler { repo: d.product_repo.clone() }
});
