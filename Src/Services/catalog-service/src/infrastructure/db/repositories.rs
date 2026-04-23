use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ddd_shared_kernel::{AppError, Page, PageRequest};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection,
    EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};
use uuid::Uuid;

use crate::domain::entities::{Brand, CountryPricing, Product, ProductCategory, ProductImage, ProductVariant, TaxConfiguration};
use crate::domain::ids::*;
use crate::domain::repositories::*;
use crate::infrastructure::db::models::*;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn db_err(e: sea_orm::DbErr) -> AppError { AppError::internal(e.to_string()) }

fn to_utc(dt: sea_orm::prelude::DateTimeWithTimeZone) -> DateTime<Utc> {
    dt.with_timezone(&Utc)
}
fn opt_to_utc(dt: Option<sea_orm::prelude::DateTimeWithTimeZone>) -> Option<DateTime<Utc>> {
    dt.map(to_utc)
}
fn from_utc(dt: DateTime<Utc>) -> sea_orm::prelude::DateTimeWithTimeZone { dt.fixed_offset() }
fn opt_from_utc(dt: Option<DateTime<Utc>>) -> Option<sea_orm::prelude::DateTimeWithTimeZone> {
    dt.map(|d| d.fixed_offset())
}

// ── Model-to-domain mappers ───────────────────────────────────────────────────

fn m2variant(m: product_variant::Model) -> ProductVariant {
    ProductVariant {
        id:                  ProductVariantId::from_uuid(m.id),
        product_id:          ProductId::from_uuid(m.product_id),
        sku:                 m.sku,
        description:         m.description,
        price_override:      m.price_override,
        cost_price_override: m.cost_price_override,
        is_active:           m.is_active,
        is_default:          m.is_default,
        barcode:             m.barcode,
        barcode_type:        m.barcode_type,
        weight_grams:        m.weight_grams,
        width_cm:            m.width_cm,
        height_cm:           m.height_cm,
        depth_cm:            m.depth_cm,
        attributes:          serde_json::from_value(m.attributes).unwrap_or_default(),
    }
}

fn m2image(m: product_image::Model) -> ProductImage {
    ProductImage {
        id:         ProductImageId::from_uuid(m.id),
        product_id: ProductId::from_uuid(m.product_id),
        url:        m.url,
        is_main:    m.is_main,
        sort_order: m.sort_order,
        alt_text:   m.alt_text,
    }
}

fn m2pricing(m: country_pricing::Model) -> CountryPricing {
    CountryPricing {
        id:             PricingId::from_uuid(m.id),
        product_id:     ProductId::from_uuid(m.product_id),
        country_code:   m.country_code,
        price:          m.price,
        effective_date: to_utc(m.effective_date),
    }
}

fn m2product(
    m:        product::Model,
    variants: Vec<ProductVariant>,
    images:   Vec<ProductImage>,
    pricing:  Vec<CountryPricing>,
) -> Product {
    Product {
        id:                           ProductId::from_uuid(m.id),
        sku:                          m.sku,
        name:                         m.name,
        description:                  m.description,
        slug:                         m.slug,
        category_id:                  m.category_id,
        weight_grams:                 m.weight_grams,
        width_cm:                     m.width_cm,
        height_cm:                    m.height_cm,
        depth_cm:                     m.depth_cm,
        brand_id:                     m.brand_id,
        base_price:                   m.base_price,
        cost_price:                   m.cost_price,
        is_taxable:                   m.is_taxable,
        is_discontinued:              m.is_discontinued,
        discontinued_at:              opt_to_utc(m.discontinued_at),
        is_inventory_tracked:         m.is_inventory_tracked,
        specifications:               serde_json::from_value(m.specifications).unwrap_or_default(),
        tags:                         serde_json::from_value(m.tags).unwrap_or_default(),
        assigned_tax_config_ids:      serde_json::from_value(m.assigned_tax_config_ids).unwrap_or_default(),
        average_rating:               m.average_rating,
        total_reviews:                m.total_reviews,
        active_promotion_id:          m.active_promotion_id,
        active_promotion_name:        m.active_promotion_name,
        promotion_discount_percentage: m.promotion_discount_percentage,
        promotion_valid_until:        opt_to_utc(m.promotion_valid_until),
        created_at:                   to_utc(m.created_at),
        created_by:                   m.created_by,
        updated_at:                   opt_to_utc(m.updated_at),
        updated_by:                   m.updated_by,
        variants,
        images,
        country_pricing: pricing,
    }
}

fn m2category(m: category::Model) -> ProductCategory {
    ProductCategory {
        id:                 CategoryId(m.id),
        name:               m.name,
        description:        m.description,
        slug:               m.slug,
        parent_category_id: m.parent_category_id,
        image_url:          m.image_url,
        is_active:          m.is_active,
        created_at:         to_utc(m.created_at),
        created_by:         m.created_by,
        updated_at:         opt_to_utc(m.updated_at),
        updated_by:         m.updated_by,
    }
}

fn m2brand(m: brand::Model) -> Brand {
    Brand {
        id:          BrandId::from_uuid(m.id),
        name:        m.name,
        description: m.description,
        slug:        m.slug,
        logo_url:    m.logo_url,
        website:     m.website,
        is_active:   m.is_active,
        created_at:  to_utc(m.created_at),
        created_by:  m.created_by,
        updated_at:  opt_to_utc(m.updated_at),
        updated_by:  m.updated_by,
    }
}

fn m2tax(m: tax_configuration::Model) -> TaxConfiguration {
    TaxConfiguration {
        id:             TaxConfigId::from_uuid(m.id),
        name:           m.name,
        code:           m.code,
        tax_type:       m.tax_type,
        location_id:    m.location_id,
        category_id:    m.category_id,
        tax_rate:       m.tax_rate,
        is_active:      m.is_active,
        effective_date: to_utc(m.effective_date),
        expiry_date:    opt_to_utc(m.expiry_date),
        created_at:     to_utc(m.created_at),
        created_by:     m.created_by,
        updated_at:     opt_to_utc(m.updated_at),
        updated_by:     m.updated_by,
    }
}

// ── PgProductRepository ───────────────────────────────────────────────────────

pub struct PgProductRepository(pub Arc<DatabaseConnection>);

impl PgProductRepository {
    async fn load_full(&self, m: product::Model) -> Result<Product, AppError> {
        let variants = product_variant::Entity::find()
            .filter(product_variant::Column::ProductId.eq(m.id))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(m2variant).collect();

        let images = product_image::Entity::find()
            .filter(product_image::Column::ProductId.eq(m.id))
            .order_by(product_image::Column::SortOrder, Order::Asc)
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(m2image).collect();

        let pricing = country_pricing::Entity::find()
            .filter(country_pricing::Column::ProductId.eq(m.id))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(m2pricing).collect();

        Ok(m2product(m, variants, images, pricing))
    }
}

#[async_trait]
impl ProductRepository for PgProductRepository {
    async fn find_by_id(&self, id: ProductId) -> Result<Option<Product>, AppError> {
        let Some(m) = product::Entity::find_by_id(id.as_uuid()).one(&*self.0).await.map_err(db_err)? else {
            return Ok(None);
        };
        Ok(Some(self.load_full(m).await?))
    }

    async fn find_by_sku(&self, sku: &str) -> Result<Option<Product>, AppError> {
        let Some(m) = product::Entity::find()
            .filter(product::Column::Sku.eq(sku.to_string()))
            .one(&*self.0).await.map_err(db_err)? else {
            return Ok(None);
        };
        Ok(Some(self.load_full(m).await?))
    }

    async fn get_paged(
        &self,
        search:      Option<&str>,
        category_id: Option<i32>,
        min_price:   Option<f64>,
        max_price:   Option<f64>,
        sort_by:     Option<&str>,
        sort_desc:   bool,
        req:         &PageRequest,
    ) -> Result<Page<Product>, AppError> {
        let mut q = product::Entity::find();

        if let Some(s) = search {
            let pat = format!("%{}%", s.to_lowercase());
            q = q.filter(
                Condition::any()
                    .add(product::Column::Name.contains(pat.clone()))
                    .add(product::Column::Sku.contains(pat))
            );
        }
        if let Some(cid) = category_id { q = q.filter(product::Column::CategoryId.eq(cid)); }
        if let Some(min) = min_price  { q = q.filter(product::Column::BasePrice.gte(min)); }
        if let Some(max) = max_price  { q = q.filter(product::Column::BasePrice.lte(max)); }

        let ord = if sort_desc { Order::Desc } else { Order::Asc };
        q = match sort_by.unwrap_or("created_at") {
            "name"       => q.order_by(product::Column::Name, ord),
            "base_price" => q.order_by(product::Column::BasePrice, ord),
            _            => q.order_by(product::Column::CreatedAt, ord),
        };

        let total = q.clone().count(&*self.0).await.map_err(db_err)?;
        let page   = req.page().max(1);
        let limit  = req.per_page().max(1);
        let offset = ((page - 1) * limit) as u64;

        let models = q.offset(offset).limit(limit as u64).all(&*self.0).await.map_err(db_err)?;
        let mut items = Vec::new();
        for m in models { items.push(self.load_full(m).await?); }

        Ok(Page::new(items, total, page, limit))
    }

    async fn save(&self, p: &Product) -> Result<(), AppError> {
        let specs = serde_json::to_value(&p.specifications).map_err(|e| AppError::internal(e.to_string()))?;
        let tags  = serde_json::to_value(&p.tags).map_err(|e| AppError::internal(e.to_string()))?;
        let tids  = serde_json::to_value(&p.assigned_tax_config_ids).map_err(|e| AppError::internal(e.to_string()))?;

        let active = product::ActiveModel {
            id:                            Set(p.id.as_uuid()),
            sku:                           Set(p.sku.clone()),
            name:                          Set(p.name.clone()),
            description:                   Set(p.description.clone()),
            slug:                          Set(p.slug.clone()),
            category_id:                   Set(p.category_id),
            weight_grams:                  Set(p.weight_grams),
            width_cm:                      Set(p.width_cm),
            height_cm:                     Set(p.height_cm),
            depth_cm:                      Set(p.depth_cm),
            brand_id:                      Set(p.brand_id),
            base_price:                    Set(p.base_price),
            cost_price:                    Set(p.cost_price),
            is_taxable:                    Set(p.is_taxable),
            is_discontinued:               Set(p.is_discontinued),
            discontinued_at:               Set(opt_from_utc(p.discontinued_at)),
            is_inventory_tracked:          Set(p.is_inventory_tracked),
            specifications:                Set(specs),
            tags:                          Set(tags),
            assigned_tax_config_ids:       Set(tids),
            average_rating:                Set(p.average_rating),
            total_reviews:                 Set(p.total_reviews),
            active_promotion_id:           Set(p.active_promotion_id),
            active_promotion_name:         Set(p.active_promotion_name.clone()),
            promotion_discount_percentage: Set(p.promotion_discount_percentage),
            promotion_valid_until:         Set(opt_from_utc(p.promotion_valid_until)),
            created_at:                    Set(from_utc(p.created_at)),
            created_by:                    Set(p.created_by.clone()),
            updated_at:                    Set(opt_from_utc(p.updated_at)),
            updated_by:                    Set(p.updated_by.clone()),
        };
        product::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(product::Column::Id)
                    .update_columns([
                        product::Column::Sku, product::Column::Name, product::Column::Description,
                        product::Column::Slug, product::Column::CategoryId, product::Column::WeightGrams,
                        product::Column::WidthCm, product::Column::HeightCm, product::Column::DepthCm,
                        product::Column::BrandId, product::Column::BasePrice, product::Column::CostPrice,
                        product::Column::IsTaxable, product::Column::IsDiscontinued,
                        product::Column::DiscontinuedAt, product::Column::IsInventoryTracked,
                        product::Column::Specifications, product::Column::Tags,
                        product::Column::AssignedTaxConfigIds, product::Column::AverageRating,
                        product::Column::TotalReviews, product::Column::ActivePromotionId,
                        product::Column::ActivePromotionName,
                        product::Column::PromotionDiscountPercentage,
                        product::Column::PromotionValidUntil, product::Column::UpdatedAt,
                        product::Column::UpdatedBy,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;

        // Sync variants: delete removed, upsert remaining
        let existing_variant_ids: Vec<Uuid> = product_variant::Entity::find()
            .filter(product_variant::Column::ProductId.eq(p.id.as_uuid()))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(|m| m.id).collect();
        let current_variant_ids: std::collections::HashSet<Uuid> = p.variants.iter().map(|v| v.id.as_uuid()).collect();
        for old_id in &existing_variant_ids {
            if !current_variant_ids.contains(old_id) {
                product_variant::Entity::delete_by_id(*old_id).exec(&*self.0).await.map_err(db_err)?;
            }
        }
        for v in &p.variants {
            let attrs = serde_json::to_value(&v.attributes).map_err(|e| AppError::internal(e.to_string()))?;
            let am = product_variant::ActiveModel {
                id:                  Set(v.id.as_uuid()),
                product_id:          Set(p.id.as_uuid()),
                sku:                 Set(v.sku.clone()),
                description:         Set(v.description.clone()),
                price_override:      Set(v.price_override),
                cost_price_override: Set(v.cost_price_override),
                is_active:           Set(v.is_active),
                is_default:          Set(v.is_default),
                barcode:             Set(v.barcode.clone()),
                barcode_type:        Set(v.barcode_type.clone()),
                weight_grams:        Set(v.weight_grams),
                width_cm:            Set(v.width_cm),
                height_cm:           Set(v.height_cm),
                depth_cm:            Set(v.depth_cm),
                attributes:          Set(attrs),
            };
            product_variant::Entity::insert(am)
                .on_conflict(
                    sea_orm::sea_query::OnConflict::column(product_variant::Column::Id)
                        .update_columns([
                            product_variant::Column::Sku, product_variant::Column::Description,
                            product_variant::Column::PriceOverride, product_variant::Column::CostPriceOverride,
                            product_variant::Column::IsActive, product_variant::Column::IsDefault,
                            product_variant::Column::Barcode, product_variant::Column::BarcodeType,
                            product_variant::Column::WeightGrams, product_variant::Column::WidthCm,
                            product_variant::Column::HeightCm, product_variant::Column::DepthCm,
                            product_variant::Column::Attributes,
                        ])
                        .to_owned(),
                )
                .exec(&*self.0).await.map_err(db_err)?;
        }

        // Sync images: delete removed, upsert remaining
        let existing_image_ids: Vec<Uuid> = product_image::Entity::find()
            .filter(product_image::Column::ProductId.eq(p.id.as_uuid()))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(|m| m.id).collect();
        let current_image_ids: std::collections::HashSet<Uuid> = p.images.iter().map(|i| i.id.as_uuid()).collect();
        for old_id in &existing_image_ids {
            if !current_image_ids.contains(old_id) {
                product_image::Entity::delete_by_id(*old_id).exec(&*self.0).await.map_err(db_err)?;
            }
        }
        for img in &p.images {
            let am = product_image::ActiveModel {
                id:         Set(img.id.as_uuid()),
                product_id: Set(p.id.as_uuid()),
                url:        Set(img.url.clone()),
                is_main:    Set(img.is_main),
                sort_order: Set(img.sort_order),
                alt_text:   Set(img.alt_text.clone()),
            };
            product_image::Entity::insert(am)
                .on_conflict(
                    sea_orm::sea_query::OnConflict::column(product_image::Column::Id)
                        .update_columns([
                            product_image::Column::Url, product_image::Column::IsMain,
                            product_image::Column::SortOrder, product_image::Column::AltText,
                        ])
                        .to_owned(),
                )
                .exec(&*self.0).await.map_err(db_err)?;
        }

        Ok(())
    }
}

// ── PgCategoryRepository ──────────────────────────────────────────────────────

pub struct PgCategoryRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl CategoryRepository for PgCategoryRepository {
    async fn find_by_id(&self, id: CategoryId) -> Result<Option<ProductCategory>, AppError> {
        Ok(category::Entity::find_by_id(id.0)
            .one(&*self.0).await.map_err(db_err)?
            .map(m2category))
    }

    async fn get_all(&self, parent_id: Option<i32>) -> Result<Vec<ProductCategory>, AppError> {
        let mut q = category::Entity::find();
        if let Some(pid) = parent_id {
            q = q.filter(category::Column::ParentCategoryId.eq(pid));
        }
        Ok(q.all(&*self.0).await.map_err(db_err)?.into_iter().map(m2category).collect())
    }

    async fn insert(&self, cat: &ProductCategory) -> Result<i32, AppError> {
        // Use raw SQL to get the SERIAL-assigned id back
        let row = self.0
            .query_one(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "INSERT INTO catalog.categories (name, description, slug, parent_category_id, image_url, is_active, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, NOW()) RETURNING id",
                [
                    cat.name.clone().into(),
                    cat.description.clone().into(),
                    cat.slug.clone().into(),
                    cat.parent_category_id.into(),
                    cat.image_url.clone().into(),
                    cat.is_active.into(),
                ],
            ))
            .await.map_err(db_err)?
            .ok_or_else(|| AppError::internal("INSERT did not return a row"))?;
        let id: i32 = row.try_get("", "id").map_err(db_err)?;
        Ok(id)
    }

    async fn save(&self, cat: &ProductCategory) -> Result<(), AppError> {
        let am = category::ActiveModel {
            id:                 Set(cat.id.0),
            name:               Set(cat.name.clone()),
            description:        Set(cat.description.clone()),
            slug:               Set(cat.slug.clone()),
            parent_category_id: Set(cat.parent_category_id),
            image_url:          Set(cat.image_url.clone()),
            is_active:          Set(cat.is_active),
            created_at:         Set(from_utc(cat.created_at)),
            created_by:         Set(cat.created_by),
            updated_at:         Set(opt_from_utc(cat.updated_at)),
            updated_by:         Set(cat.updated_by),
        };
        category::Entity::insert(am)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(category::Column::Id)
                    .update_columns([
                        category::Column::Name, category::Column::Description,
                        category::Column::Slug, category::Column::ParentCategoryId,
                        category::Column::ImageUrl, category::Column::IsActive,
                        category::Column::UpdatedAt, category::Column::UpdatedBy,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }

    async fn delete(&self, id: CategoryId) -> Result<(), AppError> {
        category::Entity::delete_by_id(id.0).exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
}

// ── PgBrandRepository ─────────────────────────────────────────────────────────

pub struct PgBrandRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl BrandRepository for PgBrandRepository {
    async fn find_by_id(&self, id: BrandId) -> Result<Option<Brand>, AppError> {
        Ok(brand::Entity::find_by_id(id.as_uuid())
            .one(&*self.0).await.map_err(db_err)?
            .map(m2brand))
    }

    async fn get_paged(
        &self,
        search:      Option<&str>,
        active_only: bool,
        req:         &PageRequest,
    ) -> Result<Page<Brand>, AppError> {
        let mut q = brand::Entity::find();
        if active_only { q = q.filter(brand::Column::IsActive.eq(true)); }
        if let Some(s) = search {
            q = q.filter(brand::Column::Name.contains(format!("%{}%", s.to_lowercase())));
        }
        q = q.order_by(brand::Column::Name, Order::Asc);
        let total = q.clone().count(&*self.0).await.map_err(db_err)?;
        let page  = req.page().max(1);
        let limit = req.per_page().max(1);
        let items = q
            .offset(((page - 1) * limit) as u64)
            .limit(limit as u64)
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(m2brand).collect();
        Ok(Page::new(items, total, page, limit))
    }

    async fn save(&self, b: &Brand) -> Result<(), AppError> {
        let am = brand::ActiveModel {
            id:          Set(b.id.as_uuid()),
            name:        Set(b.name.clone()),
            description: Set(b.description.clone()),
            slug:        Set(b.slug.clone()),
            logo_url:    Set(b.logo_url.clone()),
            website:     Set(b.website.clone()),
            is_active:   Set(b.is_active),
            created_at:  Set(from_utc(b.created_at)),
            created_by:  Set(b.created_by.clone()),
            updated_at:  Set(opt_from_utc(b.updated_at)),
            updated_by:  Set(b.updated_by.clone()),
        };
        brand::Entity::insert(am)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(brand::Column::Id)
                    .update_columns([
                        brand::Column::Name, brand::Column::Description, brand::Column::Slug,
                        brand::Column::LogoUrl, brand::Column::Website, brand::Column::IsActive,
                        brand::Column::UpdatedAt, brand::Column::UpdatedBy,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
}

// ── PgTaxConfigRepository ─────────────────────────────────────────────────────

pub struct PgTaxConfigRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl TaxConfigRepository for PgTaxConfigRepository {
    async fn find_by_id(&self, id: TaxConfigId) -> Result<Option<TaxConfiguration>, AppError> {
        Ok(tax_configuration::Entity::find_by_id(id.as_uuid())
            .one(&*self.0).await.map_err(db_err)?
            .map(m2tax))
    }

    async fn get_filtered(
        &self,
        location_id: Option<i32>,
        tax_type:    Option<&str>,
        active_only: bool,
    ) -> Result<Vec<TaxConfiguration>, AppError> {
        let mut q = tax_configuration::Entity::find();
        if let Some(lid) = location_id { q = q.filter(tax_configuration::Column::LocationId.eq(lid)); }
        if let Some(tt) = tax_type     { q = q.filter(tax_configuration::Column::TaxType.eq(tt.to_string())); }
        if active_only                 { q = q.filter(tax_configuration::Column::IsActive.eq(true)); }
        Ok(q.all(&*self.0).await.map_err(db_err)?.into_iter().map(m2tax).collect())
    }

    async fn get_applicable(
        &self,
        location_id: i32,
        category_id: Option<i32>,
    ) -> Result<Vec<TaxConfiguration>, AppError> {
        let mut q = tax_configuration::Entity::find()
            .filter(tax_configuration::Column::LocationId.eq(location_id))
            .filter(tax_configuration::Column::IsActive.eq(true));
        if let Some(cid) = category_id {
            q = q.filter(
                Condition::any()
                    .add(tax_configuration::Column::CategoryId.is_null())
                    .add(tax_configuration::Column::CategoryId.eq(cid))
            );
        }
        Ok(q.all(&*self.0).await.map_err(db_err)?.into_iter().map(m2tax).collect())
    }

    async fn save(&self, tc: &TaxConfiguration) -> Result<(), AppError> {
        let am = tax_configuration::ActiveModel {
            id:             Set(tc.id.as_uuid()),
            name:           Set(tc.name.clone()),
            code:           Set(tc.code.clone()),
            tax_type:       Set(tc.tax_type.clone()),
            location_id:    Set(tc.location_id),
            category_id:    Set(tc.category_id),
            tax_rate:       Set(tc.tax_rate),
            is_active:      Set(tc.is_active),
            effective_date: Set(from_utc(tc.effective_date)),
            expiry_date:    Set(opt_from_utc(tc.expiry_date)),
            created_at:     Set(from_utc(tc.created_at)),
            created_by:     Set(tc.created_by.clone()),
            updated_at:     Set(opt_from_utc(tc.updated_at)),
            updated_by:     Set(tc.updated_by.clone()),
        };
        tax_configuration::Entity::insert(am)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(tax_configuration::Column::Id)
                    .update_columns([
                        tax_configuration::Column::Name, tax_configuration::Column::Code,
                        tax_configuration::Column::TaxType, tax_configuration::Column::TaxRate,
                        tax_configuration::Column::IsActive, tax_configuration::Column::EffectiveDate,
                        tax_configuration::Column::ExpiryDate, tax_configuration::Column::UpdatedAt,
                        tax_configuration::Column::UpdatedBy,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }

    async fn delete(&self, id: TaxConfigId) -> Result<(), AppError> {
        tax_configuration::Entity::delete_by_id(id.as_uuid()).exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
}
