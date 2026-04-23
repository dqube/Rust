use ddd_application::{register_command_handler, register_query_handler};
use ddd_shared_kernel::AppError;

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::queries::*;
use crate::domain::entities::{Product, ProductImage, ProductVariant};

// ── CreateProduct ─────────────────────────────────────────────────────────────

register_command_handler!(CreateProduct, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |cmd: CreateProduct| {
        let repo = repo.clone();
        async move {
            let product = Product::create(
                cmd.sku, cmd.name, cmd.category_id,
                cmd.base_price, cmd.cost_price, cmd.description, cmd.is_taxable,
            )?;
            repo.save(&product).await?;
            Ok(product)
        }
    }
});

// ── UpdateProduct ─────────────────────────────────────────────────────────────

register_command_handler!(UpdateProduct, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |cmd: UpdateProduct| {
        let repo = repo.clone();
        async move {
            let mut p = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Product {} not found", cmd.id)))?;
            p.update_basic_info(cmd.name, cmd.category_id, cmd.base_price, cmd.cost_price, cmd.is_taxable, cmd.description)?;
            repo.save(&p).await?;
            Ok(p)
        }
    }
});

// ── DiscontinueProduct ────────────────────────────────────────────────────────

register_command_handler!(DiscontinueProduct, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |cmd: DiscontinueProduct| {
        let repo = repo.clone();
        async move {
            let mut p = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Product {} not found", cmd.id)))?;
            p.discontinue()?;
            repo.save(&p).await?;
            Ok(p)
        }
    }
});

// ── ReactivateProduct ─────────────────────────────────────────────────────────

register_command_handler!(ReactivateProduct, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |cmd: ReactivateProduct| {
        let repo = repo.clone();
        async move {
            let mut p = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Product {} not found", cmd.id)))?;
            p.reactivate()?;
            repo.save(&p).await?;
            Ok(p)
        }
    }
});

// ── UpdateProductPricing ──────────────────────────────────────────────────────

register_command_handler!(UpdateProductPricing, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |cmd: UpdateProductPricing| {
        let repo = repo.clone();
        async move {
            let mut p = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Product {} not found", cmd.id)))?;
            p.update_pricing(cmd.price);
            repo.save(&p).await?;
            Ok(p)
        }
    }
});

// ── AssignProductBrand ────────────────────────────────────────────────────────

register_command_handler!(AssignProductBrand, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |cmd: AssignProductBrand| {
        let repo = repo.clone();
        async move {
            let mut p = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Product {} not found", cmd.id)))?;
            p.assign_brand(cmd.brand_id);
            repo.save(&p).await?;
            Ok(p)
        }
    }
});

// ── SetProductDimensions ──────────────────────────────────────────────────────

register_command_handler!(SetProductDimensions, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |cmd: SetProductDimensions| {
        let repo = repo.clone();
        async move {
            let mut p = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Product {} not found", cmd.id)))?;
            p.set_dimensions(cmd.weight_grams, cmd.width_cm, cmd.height_cm, cmd.depth_cm);
            repo.save(&p).await?;
            Ok(p)
        }
    }
});

// ── SetProductSpecifications ──────────────────────────────────────────────────

register_command_handler!(SetProductSpecifications, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |cmd: SetProductSpecifications| {
        let repo = repo.clone();
        async move {
            let mut p = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Product {} not found", cmd.id)))?;
            p.set_specifications(cmd.specs);
            repo.save(&p).await?;
            Ok(p)
        }
    }
});

// ── SetProductTags ────────────────────────────────────────────────────────────

register_command_handler!(SetProductTags, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |cmd: SetProductTags| {
        let repo = repo.clone();
        async move {
            let mut p = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Product {} not found", cmd.id)))?;
            p.set_tags(cmd.tags);
            repo.save(&p).await?;
            Ok(p)
        }
    }
});

// ── SetProductTaxConfigurations ───────────────────────────────────────────────

register_command_handler!(SetProductTaxConfigurations, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |cmd: SetProductTaxConfigurations| {
        let repo = repo.clone();
        async move {
            let mut p = repo.find_by_id(cmd.id).await?
                .ok_or_else(|| AppError::not_found(format!("Product {} not found", cmd.id)))?;
            p.set_tax_configurations(cmd.tax_config_ids);
            repo.save(&p).await?;
            Ok(p)
        }
    }
});

// ── AddProductVariant ─────────────────────────────────────────────────────────

register_command_handler!(AddProductVariant, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |cmd: AddProductVariant| {
        let repo = repo.clone();
        async move {
            let mut p = repo.find_by_id(cmd.product_id).await?
                .ok_or_else(|| AppError::not_found(format!("Product {} not found", cmd.product_id)))?;
            let variant = ProductVariant::create(
                cmd.product_id,
                cmd.sku,
                &cmd.attributes_json,
                cmd.price_override,
                cmd.description,
                cmd.cost_price_override,
                cmd.barcode,
                cmd.barcode_type,
                cmd.weight_grams,
                cmd.width_cm,
                cmd.height_cm,
                cmd.depth_cm,
            );
            p.add_variant(variant);
            repo.save(&p).await?;
            Ok(p)
        }
    }
});

// ── UpdateProductVariant ──────────────────────────────────────────────────────

register_command_handler!(UpdateProductVariant, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |cmd: UpdateProductVariant| {
        let repo = repo.clone();
        async move {
            let mut p = repo.find_by_id(cmd.product_id).await?
                .ok_or_else(|| AppError::not_found(format!("Product {} not found", cmd.product_id)))?;
            p.update_variant(
                cmd.variant_id, cmd.sku, &cmd.attributes_json,
                cmd.price_override, cmd.description, cmd.is_active,
                cmd.cost_price_override, cmd.barcode, cmd.barcode_type,
                cmd.weight_grams, cmd.width_cm, cmd.height_cm, cmd.depth_cm,
            )?;
            repo.save(&p).await?;
            Ok(p)
        }
    }
});

// ── RemoveProductVariant ──────────────────────────────────────────────────────

register_command_handler!(RemoveProductVariant, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |cmd: RemoveProductVariant| {
        let repo = repo.clone();
        async move {
            let mut p = repo.find_by_id(cmd.product_id).await?
                .ok_or_else(|| AppError::not_found(format!("Product {} not found", cmd.product_id)))?;
            p.remove_variant(cmd.variant_id)?;
            repo.save(&p).await?;
            Ok(p)
        }
    }
});

// ── SetDefaultVariant ─────────────────────────────────────────────────────────

register_command_handler!(SetDefaultVariant, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |cmd: SetDefaultVariant| {
        let repo = repo.clone();
        async move {
            let mut p = repo.find_by_id(cmd.product_id).await?
                .ok_or_else(|| AppError::not_found(format!("Product {} not found", cmd.product_id)))?;
            p.set_default_variant(cmd.variant_id)?;
            repo.save(&p).await?;
            Ok(p)
        }
    }
});

// ── RequestProductImageUploadUrl ──────────────────────────────────────────────

register_command_handler!(RequestProductImageUploadUrl, AppDeps, |d: &AppDeps| {
    let repo    = d.product_repo.clone();
    let storage = d.blob_storage.clone();
    let bucket  = d.blob_bucket.clone();
    let ttl     = d.presign_ttl_secs;
    move |cmd: RequestProductImageUploadUrl| {
        let repo    = repo.clone();
        let storage = storage.clone();
        let bucket  = bucket.clone();
        async move {
            repo.find_by_id(cmd.product_id).await?
                .ok_or_else(|| AppError::not_found(format!("Product {} not found", cmd.product_id)))?;
            let ext = cmd.file_name.rsplit('.').next().unwrap_or("bin");
            let object_name = format!("catalog/products/{}/{}.{}", cmd.product_id, uuid::Uuid::new_v4(), ext);
            let presigned = storage
                .presigned_put(&bucket, &object_name, &cmd.content_type, std::time::Duration::from_secs(ttl))
                .await
                .map_err(|e| AppError::internal(e.to_string()))?;
            Ok((presigned.url, object_name, presigned.expires_at.to_rfc3339()))
        }
    }
});

// ── ConfirmProductImageUpload ─────────────────────────────────────────────────

register_command_handler!(ConfirmProductImageUpload, AppDeps, |d: &AppDeps| {
    let repo    = d.product_repo.clone();
    let storage = d.blob_storage.clone();
    let bucket  = d.blob_bucket.clone();
    let ttl     = d.presign_ttl_secs;
    move |cmd: ConfirmProductImageUpload| {
        let repo    = repo.clone();
        let storage = storage.clone();
        let bucket  = bucket.clone();
        async move {
            let mut p = repo.find_by_id(cmd.product_id).await?
                .ok_or_else(|| AppError::not_found(format!("Product {} not found", cmd.product_id)))?;
            let presigned = storage
                .presigned_get(&bucket, &cmd.object_name, std::time::Duration::from_secs(ttl))
                .await
                .map_err(|e| AppError::internal(e.to_string()))?;
            let image = ProductImage::create(
                cmd.product_id, presigned.url, cmd.is_main, cmd.sort_order, cmd.alt_text,
            );
            p.add_image(image);
            repo.save(&p).await?;
            Ok(p)
        }
    }
});

// ── DeleteProductImage ────────────────────────────────────────────────────────

register_command_handler!(DeleteProductImage, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |cmd: DeleteProductImage| {
        let repo = repo.clone();
        async move {
            let mut p = repo.find_by_id(cmd.product_id).await?
                .ok_or_else(|| AppError::not_found(format!("Product {} not found", cmd.product_id)))?;
            p.remove_image(cmd.image_id)?;
            repo.save(&p).await?;
            Ok(p)
        }
    }
});

// ── GetProduct ────────────────────────────────────────────────────────────────

register_query_handler!(GetProduct, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |q: GetProduct| {
        let repo = repo.clone();
        async move { repo.find_by_id(q.id).await }
    }
});

// ── ListProducts ──────────────────────────────────────────────────────────────

register_query_handler!(ListProducts, AppDeps, |d: &AppDeps| {
    let repo = d.product_repo.clone();
    move |q: ListProducts| {
        let repo = repo.clone();
        async move {
            repo.get_paged(
                q.search.as_deref(),
                q.category_id,
                q.min_price,
                q.max_price,
                q.sort_by.as_deref(),
                q.sort_desc,
                &q.req,
            ).await
        }
    }
});
