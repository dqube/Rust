use std::sync::Arc;

use ddd_api::grpc::GrpcErrorExt;
use ddd_application::Mediator;
use ddd_shared_kernel::PageRequest;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::application::commands::*;
use crate::application::queries::*;
use crate::domain::entities::*;
use crate::domain::ids::*;
use crate::proto::catalog_service_server::{CatalogService, CatalogServiceServer};
use crate::proto::*;

pub struct CatalogGrpcService {
    mediator: Arc<Mediator>,
}

impl CatalogGrpcService {
    pub fn new(mediator: Arc<Mediator>) -> Self { Self { mediator } }
    pub fn into_server(self) -> CatalogServiceServer<Self> { CatalogServiceServer::new(self) }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_uuid(s: &str, label: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("invalid {label}: {s}")))
}

fn opt_str(s: &str) -> Option<String> {
    if s.is_empty() { None } else { Some(s.to_string()) }
}

fn parse_datetime(s: &str, label: &str) -> Result<chrono::DateTime<chrono::Utc>, Status> {
    if s.is_empty() {
        return Ok(chrono::Utc::now());
    }
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&chrono::Utc))
        .map_err(|_| Status::invalid_argument(format!("invalid datetime for {label}: {s}")))
}

fn opt_datetime(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    if s.is_empty() { return None; }
    chrono::DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&chrono::Utc))
}

// ── Message converters ────────────────────────────────────────────────────────

fn to_product_message(p: Product) -> ProductMessage {
    ProductMessage {
        product_id:              p.id.to_string(),
        sku:                     p.sku,
        name:                    p.name,
        description:             p.description.unwrap_or_default(),
        slug:                    p.slug.unwrap_or_default(),
        category_id:             p.category_id,
        base_price:              p.base_price,
        cost_price:              p.cost_price,
        is_taxable:              p.is_taxable,
        is_discontinued:         p.is_discontinued,
        is_inventory_tracked:    p.is_inventory_tracked,
        brand_id:                p.brand_id.map(|u| u.to_string()).unwrap_or_default(),
        weight_grams:            p.weight_grams.unwrap_or(0),
        width_cm:                p.width_cm.unwrap_or(0),
        height_cm:               p.height_cm.unwrap_or(0),
        depth_cm:                p.depth_cm.unwrap_or(0),
        specifications_json:     serde_json::to_string(&p.specifications).unwrap_or_default(),
        tags:                    p.tags,
        assigned_tax_config_ids: p.assigned_tax_config_ids,
        average_rating:          p.average_rating.unwrap_or(0.0),
        total_reviews:           p.total_reviews,
        variants:                p.variants.into_iter().map(to_variant_message).collect(),
        images:                  p.images.into_iter().map(to_image_message).collect(),
        created_at:              p.created_at.to_rfc3339(),
        updated_at:              p.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
    }
}

fn to_variant_message(v: ProductVariant) -> ProductVariantMessage {
    ProductVariantMessage {
        variant_id:          v.id.to_string(),
        sku:                 v.sku,
        description:         v.description.unwrap_or_default(),
        price_override:      v.price_override.unwrap_or(0.0),
        cost_price_override: v.cost_price_override.unwrap_or(0.0),
        is_active:           v.is_active,
        is_default:          v.is_default,
        barcode:             v.barcode.unwrap_or_default(),
        barcode_type:        v.barcode_type.unwrap_or_default(),
        weight_grams:        v.weight_grams.unwrap_or(0),
        width_cm:            v.width_cm.unwrap_or(0),
        height_cm:           v.height_cm.unwrap_or(0),
        depth_cm:            v.depth_cm.unwrap_or(0),
        attributes_json:     serde_json::to_string(&v.attributes).unwrap_or_default(),
    }
}

fn to_image_message(i: ProductImage) -> ProductImageMessage {
    ProductImageMessage {
        image_id:   i.id.to_string(),
        url:        i.url,
        is_main:    i.is_main,
        sort_order: i.sort_order,
        alt_text:   i.alt_text.unwrap_or_default(),
    }
}

fn to_category_message(c: ProductCategory) -> CategoryMessage {
    CategoryMessage {
        category_id:        c.id.0,
        name:               c.name,
        description:        c.description.unwrap_or_default(),
        slug:               c.slug.unwrap_or_default(),
        parent_category_id: c.parent_category_id.unwrap_or(0),
        has_parent:         c.parent_category_id.is_some(),
        image_url:          c.image_url.unwrap_or_default(),
        is_active:          c.is_active,
        created_at:         c.created_at.to_rfc3339(),
        updated_at:         c.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
    }
}

fn to_brand_message(b: Brand) -> BrandMessage {
    BrandMessage {
        brand_id:    b.id.to_string(),
        name:        b.name,
        description: b.description.unwrap_or_default(),
        slug:        b.slug.unwrap_or_default(),
        logo_url:    b.logo_url.unwrap_or_default(),
        website:     b.website.unwrap_or_default(),
        is_active:   b.is_active,
        created_at:  b.created_at.to_rfc3339(),
        updated_at:  b.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
    }
}

fn to_tax_message(tc: TaxConfiguration) -> TaxConfigMessage {
    TaxConfigMessage {
        tax_config_id:  tc.id.to_string(),
        name:           tc.name,
        code:           tc.code,
        tax_type:       tc.tax_type,
        location_id:    tc.location_id,
        category_id:    tc.category_id.unwrap_or(0),
        has_category:   tc.category_id.is_some(),
        tax_rate:       tc.tax_rate,
        is_active:      tc.is_active,
        effective_date: tc.effective_date.to_rfc3339(),
        expiry_date:    tc.expiry_date.map(|d| d.to_rfc3339()).unwrap_or_default(),
        created_at:     tc.created_at.to_rfc3339(),
        updated_at:     tc.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
    }
}

fn to_product_summary(p: &Product) -> ProductSummary {
    ProductSummary {
        product_id:      p.id.to_string(),
        sku:             p.sku.clone(),
        name:            p.name.clone(),
        category_id:     p.category_id,
        base_price:      p.base_price,
        is_taxable:      p.is_taxable,
        is_discontinued: p.is_discontinued,
        slug:            p.slug.clone().unwrap_or_default(),
        created_at:      p.created_at.to_rfc3339(),
    }
}

// ── CatalogService implementation ─────────────────────────────────────────────

#[tonic::async_trait]
impl CatalogService for CatalogGrpcService {

    // ── Products ────────────────────────────────────────────────────────────

    async fn create_product(&self, req: Request<CreateProductRequest>) -> Result<Response<ProductMessage>, Status> {
        let r = req.into_inner();
        let p = self.mediator.send(CreateProduct {
            sku:         r.sku,
            name:        r.name,
            category_id: r.category_id,
            base_price:  r.base_price,
            cost_price:  r.cost_price,
            description: opt_str(&r.description),
            is_taxable:  r.is_taxable,
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_product_message(p)))
    }

    async fn get_product(&self, req: Request<GetProductRequest>) -> Result<Response<ProductMessage>, Status> {
        let r = req.into_inner();
        let id = parse_uuid(&r.product_id, "product_id")?;
        let p = self.mediator.query(GetProduct { id: ProductId::from_uuid(id) }).await.map_err(|e| e.to_grpc_status())?
            .ok_or_else(|| Status::not_found(format!("Product {} not found", r.product_id)))?;
        Ok(Response::new(to_product_message(p)))
    }

    async fn update_product(&self, req: Request<UpdateProductRequest>) -> Result<Response<ProductMessage>, Status> {
        let r = req.into_inner();
        let id = parse_uuid(&r.product_id, "product_id")?;
        let p = self.mediator.send(UpdateProduct {
            id:          ProductId::from_uuid(id),
            name:        r.name,
            category_id: r.category_id,
            base_price:  r.base_price,
            cost_price:  r.cost_price,
            is_taxable:  r.is_taxable,
            description: opt_str(&r.description),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_product_message(p)))
    }

    async fn discontinue_product(&self, req: Request<ProductIdRequest>) -> Result<Response<ProductMessage>, Status> {
        let id = parse_uuid(&req.into_inner().product_id, "product_id")?;
        let p = self.mediator.send(DiscontinueProduct { id: ProductId::from_uuid(id) }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_product_message(p)))
    }

    async fn reactivate_product(&self, req: Request<ProductIdRequest>) -> Result<Response<ProductMessage>, Status> {
        let id = parse_uuid(&req.into_inner().product_id, "product_id")?;
        let p = self.mediator.send(ReactivateProduct { id: ProductId::from_uuid(id) }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_product_message(p)))
    }

    async fn update_product_pricing(&self, req: Request<UpdateProductPricingRequest>) -> Result<Response<ProductMessage>, Status> {
        let r = req.into_inner();
        let id = parse_uuid(&r.product_id, "product_id")?;
        let p = self.mediator.send(UpdateProductPricing { id: ProductId::from_uuid(id), price: r.price }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_product_message(p)))
    }

    async fn list_products(&self, req: Request<ListProductsRequest>) -> Result<Response<ListProductsResponse>, Status> {
        let r = req.into_inner();
        let page_req = PageRequest::new(r.page.max(1), r.per_page.max(1));
        let page = self.mediator.query(ListProducts {
            search:      opt_str(&r.search),
            category_id: if r.has_category { Some(r.category_id) } else { None },
            min_price:   if r.has_price_range { Some(r.min_price) } else { None },
            max_price:   if r.has_price_range { Some(r.max_price) } else { None },
            sort_by:     opt_str(&r.sort_by),
            sort_desc:   r.sort_desc,
            req:         page_req,
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ListProductsResponse {
            items:       page.items().iter().map(to_product_summary).collect(),
            total:       page.total() as u64,
            total_pages: page.total_pages() as u32,
            page:        page.page() as u32,
            per_page:    page.per_page() as u32,
        }))
    }

    async fn assign_product_brand(&self, req: Request<AssignProductBrandRequest>) -> Result<Response<ProductMessage>, Status> {
        let r = req.into_inner();
        let id       = parse_uuid(&r.product_id, "product_id")?;
        let brand_id = if r.brand_id.is_empty() { None } else { Some(parse_uuid(&r.brand_id, "brand_id")?) };
        let p = self.mediator.send(AssignProductBrand { id: ProductId::from_uuid(id), brand_id }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_product_message(p)))
    }

    async fn add_product_variant(&self, req: Request<AddProductVariantRequest>) -> Result<Response<ProductMessage>, Status> {
        let r = req.into_inner();
        let pid = parse_uuid(&r.product_id, "product_id")?;
        let p = self.mediator.send(AddProductVariant {
            product_id:          ProductId::from_uuid(pid),
            sku:                 r.sku,
            attributes_json:     r.attributes_json,
            price_override:      if r.has_price_override { Some(r.price_override) } else { None },
            description:         opt_str(&r.description),
            cost_price_override: if r.has_cost_price_override { Some(r.cost_price_override) } else { None },
            barcode:             opt_str(&r.barcode),
            barcode_type:        opt_str(&r.barcode_type),
            weight_grams:        if r.has_weight { Some(r.weight_grams) } else { None },
            width_cm:            if r.has_dimensions { Some(r.width_cm) } else { None },
            height_cm:           if r.has_dimensions { Some(r.height_cm) } else { None },
            depth_cm:            if r.has_dimensions { Some(r.depth_cm) } else { None },
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_product_message(p)))
    }

    async fn update_product_variant(&self, req: Request<UpdateProductVariantRequest>) -> Result<Response<ProductMessage>, Status> {
        let r = req.into_inner();
        let pid = parse_uuid(&r.product_id, "product_id")?;
        let vid = parse_uuid(&r.variant_id, "variant_id")?;
        let p = self.mediator.send(UpdateProductVariant {
            product_id:          ProductId::from_uuid(pid),
            variant_id:          ProductVariantId::from_uuid(vid),
            sku:                 r.sku,
            attributes_json:     r.attributes_json,
            price_override:      if r.has_price_override { Some(r.price_override) } else { None },
            description:         opt_str(&r.description),
            is_active:           r.is_active,
            cost_price_override: if r.has_cost_price_override { Some(r.cost_price_override) } else { None },
            barcode:             if r.has_barcode { opt_str(&r.barcode) } else { None },
            barcode_type:        if r.has_barcode { opt_str(&r.barcode_type) } else { None },
            weight_grams:        if r.has_weight { Some(r.weight_grams) } else { None },
            width_cm:            if r.has_dimensions { Some(r.width_cm) } else { None },
            height_cm:           if r.has_dimensions { Some(r.height_cm) } else { None },
            depth_cm:            if r.has_dimensions { Some(r.depth_cm) } else { None },
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_product_message(p)))
    }

    async fn remove_product_variant(&self, req: Request<RemoveProductVariantRequest>) -> Result<Response<ProductMessage>, Status> {
        let r = req.into_inner();
        let pid = parse_uuid(&r.product_id, "product_id")?;
        let vid = parse_uuid(&r.variant_id, "variant_id")?;
        let p = self.mediator.send(RemoveProductVariant {
            product_id: ProductId::from_uuid(pid),
            variant_id: ProductVariantId::from_uuid(vid),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_product_message(p)))
    }

    async fn set_default_variant(&self, req: Request<SetDefaultVariantRequest>) -> Result<Response<ProductMessage>, Status> {
        let r = req.into_inner();
        let pid = parse_uuid(&r.product_id, "product_id")?;
        let vid = parse_uuid(&r.variant_id, "variant_id")?;
        let p = self.mediator.send(SetDefaultVariant {
            product_id: ProductId::from_uuid(pid),
            variant_id: ProductVariantId::from_uuid(vid),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_product_message(p)))
    }

    async fn set_product_dimensions(&self, req: Request<SetProductDimensionsRequest>) -> Result<Response<ProductMessage>, Status> {
        let r = req.into_inner();
        let id = parse_uuid(&r.product_id, "product_id")?;
        let p = self.mediator.send(SetProductDimensions {
            id:          ProductId::from_uuid(id),
            weight_grams: if r.has_weight { Some(r.weight_grams) } else { None },
            width_cm:    if r.has_dims { Some(r.width_cm) } else { None },
            height_cm:   if r.has_dims { Some(r.height_cm) } else { None },
            depth_cm:    if r.has_dims { Some(r.depth_cm) } else { None },
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_product_message(p)))
    }

    async fn set_product_specifications(&self, req: Request<SetProductSpecificationsRequest>) -> Result<Response<ProductMessage>, Status> {
        let r = req.into_inner();
        let id = parse_uuid(&r.product_id, "product_id")?;
        let specs: serde_json::Value = serde_json::from_str(&r.specifications_json)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        let p = self.mediator.send(SetProductSpecifications { id: ProductId::from_uuid(id), specs }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_product_message(p)))
    }

    async fn set_product_tags(&self, req: Request<SetProductTagsRequest>) -> Result<Response<ProductMessage>, Status> {
        let r = req.into_inner();
        let id = parse_uuid(&r.product_id, "product_id")?;
        let p = self.mediator.send(SetProductTags { id: ProductId::from_uuid(id), tags: r.tags }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_product_message(p)))
    }

    async fn set_product_tax_configurations(&self, req: Request<SetProductTaxConfigurationsRequest>) -> Result<Response<ProductMessage>, Status> {
        let r = req.into_inner();
        let id = parse_uuid(&r.product_id, "product_id")?;
        let p = self.mediator.send(SetProductTaxConfigurations {
            id: ProductId::from_uuid(id),
            tax_config_ids: r.tax_config_ids,
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_product_message(p)))
    }

    async fn request_product_image_upload_url(&self, req: Request<RequestProductImageUploadUrlRequest>) -> Result<Response<RequestProductImageUploadUrlResponse>, Status> {
        let r = req.into_inner();
        let id = parse_uuid(&r.product_id, "product_id")?;
        let (url, object_name, expires_at) = self.mediator.send(RequestProductImageUploadUrl {
            product_id:   ProductId::from_uuid(id),
            file_name:    r.file_name,
            content_type: r.content_type,
            is_main:      r.is_main,
            sort_order:   r.sort_order,
            alt_text:     opt_str(&r.alt_text),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(RequestProductImageUploadUrlResponse { upload_url: url, object_name, expires_at }))
    }

    async fn confirm_product_image_upload(&self, req: Request<ConfirmProductImageUploadRequest>) -> Result<Response<ProductMessage>, Status> {
        let r = req.into_inner();
        let id = parse_uuid(&r.product_id, "product_id")?;
        let p = self.mediator.send(ConfirmProductImageUpload {
            product_id:   ProductId::from_uuid(id),
            object_name:  r.object_name,
            file_name:    r.file_name,
            content_type: r.content_type,
            is_main:      r.is_main,
            sort_order:   r.sort_order,
            alt_text:     opt_str(&r.alt_text),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_product_message(p)))
    }

    async fn delete_product_image(&self, req: Request<DeleteProductImageRequest>) -> Result<Response<ProductMessage>, Status> {
        let r = req.into_inner();
        let pid = parse_uuid(&r.product_id, "product_id")?;
        let iid = parse_uuid(&r.image_id, "image_id")?;
        let p = self.mediator.send(DeleteProductImage {
            product_id: ProductId::from_uuid(pid),
            image_id:   ProductImageId::from_uuid(iid),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_product_message(p)))
    }

    // ── Categories ──────────────────────────────────────────────────────────

    async fn create_category(&self, req: Request<CreateCategoryRequest>) -> Result<Response<CategoryMessage>, Status> {
        let r = req.into_inner();
        let c = self.mediator.send(CreateCategory {
            name:               r.name,
            description:        opt_str(&r.description),
            parent_category_id: if r.has_parent { Some(r.parent_category_id) } else { None },
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_category_message(c)))
    }

    async fn get_category(&self, req: Request<GetCategoryRequest>) -> Result<Response<CategoryMessage>, Status> {
        let id = req.into_inner().category_id;
        let c = self.mediator.query(GetCategory { id: CategoryId(id) }).await.map_err(|e| e.to_grpc_status())?
            .ok_or_else(|| Status::not_found(format!("Category {id} not found")))?;
        Ok(Response::new(to_category_message(c)))
    }

    async fn list_categories(&self, req: Request<ListCategoriesRequest>) -> Result<Response<ListCategoriesResponse>, Status> {
        let r = req.into_inner();
        let cats = self.mediator.query(ListCategories {
            parent_id: if r.has_parent { Some(r.parent_category_id) } else { None },
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ListCategoriesResponse {
            categories: cats.into_iter().map(to_category_message).collect(),
        }))
    }

    async fn update_category(&self, req: Request<UpdateCategoryRequest>) -> Result<Response<CategoryMessage>, Status> {
        let r = req.into_inner();
        let c = self.mediator.send(UpdateCategory {
            id:                 CategoryId(r.category_id),
            name:               r.name,
            description:        opt_str(&r.description),
            parent_category_id: if r.has_parent { Some(r.parent_category_id) } else { None },
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_category_message(c)))
    }

    async fn delete_category(&self, req: Request<DeleteCategoryRequest>) -> Result<Response<Empty>, Status> {
        let id = req.into_inner().category_id;
        self.mediator.send(DeleteCategory { id: CategoryId(id) }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }

    async fn request_category_image_upload_url(&self, req: Request<RequestCategoryImageUploadUrlRequest>) -> Result<Response<RequestCategoryImageUploadUrlResponse>, Status> {
        let r = req.into_inner();
        let (url, object_name, expires_at) = self.mediator.send(RequestCategoryImageUploadUrl {
            category_id:  CategoryId(r.category_id),
            file_name:    r.file_name,
            content_type: r.content_type,
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(RequestCategoryImageUploadUrlResponse { upload_url: url, object_name, expires_at }))
    }

    async fn confirm_category_image_upload(&self, req: Request<ConfirmCategoryImageUploadRequest>) -> Result<Response<CategoryMessage>, Status> {
        let r = req.into_inner();
        // Build a public GET URL from the object_name using the blob storage base URL
        // For simplicity, we pass object_name as public_url — the handler will resolve it via presigned_get
        let c = self.mediator.send(ConfirmCategoryImageUpload {
            category_id: CategoryId(r.category_id),
            object_name: r.object_name.clone(),
            public_url:  r.object_name,
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_category_message(c)))
    }

    // ── Brands ──────────────────────────────────────────────────────────────

    async fn create_brand(&self, req: Request<CreateBrandRequest>) -> Result<Response<BrandMessage>, Status> {
        let r = req.into_inner();
        let b = self.mediator.send(CreateBrand {
            name:        r.name,
            description: opt_str(&r.description),
            website:     opt_str(&r.website),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_brand_message(b)))
    }

    async fn get_brand(&self, req: Request<GetBrandRequest>) -> Result<Response<BrandMessage>, Status> {
        let id = parse_uuid(&req.into_inner().brand_id, "brand_id")?;
        let b = self.mediator.query(GetBrand { id: BrandId::from_uuid(id) }).await.map_err(|e| e.to_grpc_status())?
            .ok_or_else(|| Status::not_found(format!("Brand {id} not found")))?;
        Ok(Response::new(to_brand_message(b)))
    }

    async fn list_brands(&self, req: Request<ListBrandsRequest>) -> Result<Response<ListBrandsResponse>, Status> {
        let r = req.into_inner();
        let page_req = PageRequest::new(r.page.max(1), r.per_page.max(1));
        let page = self.mediator.query(ListBrands {
            search:      opt_str(&r.search),
            active_only: r.active_only,
            req:         page_req,
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ListBrandsResponse {
            total:       page.total() as u64,
            total_pages: page.total_pages() as u32,
            page:        page.page() as u32,
            per_page:    page.per_page() as u32,
            items:       page.into_items().into_iter().map(to_brand_message).collect(),
        }))
    }

    async fn update_brand(&self, req: Request<UpdateBrandRequest>) -> Result<Response<BrandMessage>, Status> {
        let r = req.into_inner();
        let id = parse_uuid(&r.brand_id, "brand_id")?;
        let b = self.mediator.send(UpdateBrand {
            id:          BrandId::from_uuid(id),
            name:        r.name,
            description: opt_str(&r.description),
            website:     opt_str(&r.website),
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_brand_message(b)))
    }

    async fn activate_brand(&self, req: Request<BrandIdRequest>) -> Result<Response<BrandMessage>, Status> {
        let id = parse_uuid(&req.into_inner().brand_id, "brand_id")?;
        let b = self.mediator.send(ActivateBrand { id: BrandId::from_uuid(id) }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_brand_message(b)))
    }

    async fn deactivate_brand(&self, req: Request<BrandIdRequest>) -> Result<Response<BrandMessage>, Status> {
        let id = parse_uuid(&req.into_inner().brand_id, "brand_id")?;
        let b = self.mediator.send(DeactivateBrand { id: BrandId::from_uuid(id) }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_brand_message(b)))
    }

    // ── Tax Configurations ──────────────────────────────────────────────────

    async fn create_tax_configuration(&self, req: Request<CreateTaxConfigurationRequest>) -> Result<Response<TaxConfigMessage>, Status> {
        let r = req.into_inner();
        let effective = parse_datetime(&r.effective_date, "effective_date")?;
        let expiry    = if r.has_expiry { opt_datetime(&r.expiry_date) } else { None };
        let tc = self.mediator.send(CreateTaxConfiguration {
            name:           r.name,
            code:           r.code,
            tax_type:       r.tax_type,
            tax_rate:       r.tax_rate,
            location_id:    r.location_id,
            category_id:    if r.has_category { Some(r.category_id) } else { None },
            effective_date: effective,
            expiry_date:    expiry,
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_tax_message(tc)))
    }

    async fn get_tax_configuration(&self, req: Request<TaxConfigIdRequest>) -> Result<Response<TaxConfigMessage>, Status> {
        let id = parse_uuid(&req.into_inner().tax_config_id, "tax_config_id")?;
        let tc = self.mediator.query(GetTaxConfig { id: TaxConfigId::from_uuid(id) }).await.map_err(|e| e.to_grpc_status())?
            .ok_or_else(|| Status::not_found(format!("TaxConfig {id} not found")))?;
        Ok(Response::new(to_tax_message(tc)))
    }

    async fn list_tax_configurations(&self, req: Request<ListTaxConfigurationsRequest>) -> Result<Response<ListTaxConfigurationsResponse>, Status> {
        let r = req.into_inner();
        let tcs = self.mediator.query(ListTaxConfigs {
            location_id: if r.has_location { Some(r.location_id) } else { None },
            tax_type:    if r.has_tax_type { opt_str(&r.tax_type) } else { None },
            active_only: r.active_only,
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ListTaxConfigurationsResponse {
            tax_configurations: tcs.into_iter().map(to_tax_message).collect(),
        }))
    }

    async fn update_tax_configuration(&self, req: Request<UpdateTaxConfigurationRequest>) -> Result<Response<TaxConfigMessage>, Status> {
        let r = req.into_inner();
        let id      = parse_uuid(&r.tax_config_id, "tax_config_id")?;
        let effective = parse_datetime(&r.effective_date, "effective_date")?;
        let expiry  = if r.has_expiry { opt_datetime(&r.expiry_date) } else { None };
        let tc = self.mediator.send(UpdateTaxConfiguration {
            id:             TaxConfigId::from_uuid(id),
            name:           r.name,
            code:           r.code,
            tax_type:       r.tax_type,
            tax_rate:       r.tax_rate,
            effective_date: effective,
            expiry_date:    expiry,
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_tax_message(tc)))
    }

    async fn delete_tax_configuration(&self, req: Request<TaxConfigIdRequest>) -> Result<Response<Empty>, Status> {
        let id = parse_uuid(&req.into_inner().tax_config_id, "tax_config_id")?;
        self.mediator.send(DeleteTaxConfiguration { id: TaxConfigId::from_uuid(id) }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }

    async fn activate_tax_configuration(&self, req: Request<TaxConfigIdRequest>) -> Result<Response<TaxConfigMessage>, Status> {
        let id = parse_uuid(&req.into_inner().tax_config_id, "tax_config_id")?;
        let tc = self.mediator.send(ActivateTaxConfiguration { id: TaxConfigId::from_uuid(id) }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_tax_message(tc)))
    }

    async fn deactivate_tax_configuration(&self, req: Request<TaxConfigIdRequest>) -> Result<Response<TaxConfigMessage>, Status> {
        let id = parse_uuid(&req.into_inner().tax_config_id, "tax_config_id")?;
        let tc = self.mediator.send(DeactivateTaxConfiguration { id: TaxConfigId::from_uuid(id) }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(to_tax_message(tc)))
    }

    async fn get_applicable_tax_configurations(&self, req: Request<GetApplicableTaxConfigurationsRequest>) -> Result<Response<ListTaxConfigurationsResponse>, Status> {
        let r = req.into_inner();
        let tcs = self.mediator.query(GetApplicableTaxConfigs {
            location_id: r.location_id,
            category_id: if r.has_category { Some(r.category_id) } else { None },
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ListTaxConfigurationsResponse {
            tax_configurations: tcs.into_iter().map(to_tax_message).collect(),
        }))
    }

    async fn calculate_tax(&self, req: Request<CalculateTaxRequest>) -> Result<Response<CalculateTaxResponse>, Status> {
        let r = req.into_inner();
        let (tax_amount, total_amount, applied) = self.mediator.query(CalculateTax {
            location_id: r.location_id,
            category_id: if r.has_category { Some(r.category_id) } else { None },
            amount:      r.amount,
        }).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CalculateTaxResponse {
            original_amount: r.amount,
            tax_amount,
            total_amount,
            applied: applied.into_iter().map(|(name, code, tax_type, rate, amount)| {
                AppliedTaxItem { name, code, tax_type, rate, amount }
            }).collect(),
        }))
    }
}
