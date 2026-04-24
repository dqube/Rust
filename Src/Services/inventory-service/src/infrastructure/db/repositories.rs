use std::sync::Arc;
use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use ddd_shared_kernel::{AppError, AppResult, Page, PageRequest};
use uuid::Uuid;

use crate::domain::entities::{InventoryItem, InventoryLocator, StockMovement};
use crate::domain::enums::MovementType;
use crate::domain::ids::{InventoryItemId, StockMovementId};
use crate::domain::repositories::{
    InventoryItemRepository, StockMovementFilter, StockMovementRepository,
};

use super::models;

fn to_utc(dt: sea_orm::prelude::DateTimeWithTimeZone) -> DateTime<Utc> {
    dt.with_timezone(&Utc)
}

fn opt_to_utc(
    dt: Option<sea_orm::prelude::DateTimeWithTimeZone>,
) -> Option<DateTime<Utc>> {
    dt.map(to_utc)
}

fn from_utc(dt: DateTime<Utc>) -> sea_orm::prelude::DateTimeWithTimeZone {
    dt.fixed_offset()
}

fn opt_from_utc(
    dt: Option<DateTime<Utc>>,
) -> Option<sea_orm::prelude::DateTimeWithTimeZone> {
    dt.map(|value| value.fixed_offset())
}

fn db_err(error: sea_orm::DbErr) -> AppError {
    AppError::internal(format!("database error: {error}"))
}

fn model_to_inventory_item(model: models::inventory_item::Model) -> InventoryItem {
    InventoryItem {
        id: InventoryItemId::from_uuid(model.id),
        version: 0,
        domain_events: Vec::new(),
        created_at: to_utc(model.created_at),
        updated_at: to_utc(model.updated_at),
        locator: InventoryLocator {
            product_id: model.product_id,
            product_variant_id: model.product_variant_id,
            store_id: model.store_id,
        },
        quantity: model.quantity,
        reserved_quantity: model.reserved_quantity,
        reorder_level: model.reorder_level,
        last_restock_date: opt_to_utc(model.last_restock_date),
        created_by: model.created_by,
        updated_by: model.updated_by,
    }
}

fn model_to_stock_movement(model: models::stock_movement::Model) -> StockMovement {
    StockMovement {
        id: StockMovementId::from_uuid(model.id),
        version: 0,
        domain_events: Vec::new(),
        created_at: to_utc(model.created_at),
        updated_at: to_utc(model.updated_at),
        locator: InventoryLocator {
            product_id: model.product_id,
            product_variant_id: model.product_variant_id,
            store_id: model.store_id,
        },
        quantity_change: model.quantity_change,
        movement_type: MovementType::from_str(&model.movement_type)
            .unwrap_or(MovementType::Adjustment),
        movement_date: to_utc(model.movement_date),
        employee_id: model.employee_id,
        reference_id: model.reference_id,
        notes: model.notes,
    }
}

pub struct PgInventoryItemRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl InventoryItemRepository for PgInventoryItemRepository {
    async fn find_by_id(&self, id: InventoryItemId) -> AppResult<Option<InventoryItem>> {
        let row = models::inventory_item::Entity::find_by_id(id.as_uuid())
            .one(self.0.as_ref())
            .await
            .map_err(db_err)?;
        Ok(row.map(model_to_inventory_item))
    }

    async fn save(&self, item: &InventoryItem) -> AppResult<()> {
        let active_model = models::inventory_item::ActiveModel {
            id: Set(item.id.as_uuid()),
            product_id: Set(item.locator.product_id),
            product_variant_id: Set(item.locator.product_variant_id),
            store_id: Set(item.locator.store_id),
            quantity: Set(item.quantity),
            reserved_quantity: Set(item.reserved_quantity),
            reorder_level: Set(item.reorder_level),
            last_restock_date: Set(opt_from_utc(item.last_restock_date)),
            created_at: Set(from_utc(item.created_at)),
            created_by: Set(item.created_by.clone()),
            updated_at: Set(from_utc(item.updated_at)),
            updated_by: Set(item.updated_by.clone()),
        };

        if models::inventory_item::Entity::find_by_id(item.id.as_uuid())
            .one(self.0.as_ref())
            .await
            .map_err(db_err)?
            .is_some()
        {
            active_model.update(self.0.as_ref()).await.map_err(db_err)?;
        } else {
            active_model.insert(self.0.as_ref()).await.map_err(db_err)?;
        }
        Ok(())
    }

    async fn exists(&self, locator: InventoryLocator) -> AppResult<bool> {
        let mut query = models::inventory_item::Entity::find()
            .filter(models::inventory_item::Column::StoreId.eq(locator.store_id))
            .filter(models::inventory_item::Column::ProductId.eq(locator.product_id));
        query = match locator.product_variant_id {
            Some(variant_id) => {
                query.filter(models::inventory_item::Column::ProductVariantId.eq(variant_id))
            }
            None => query.filter(models::inventory_item::Column::ProductVariantId.is_null()),
        };
        let count = query.count(self.0.as_ref()).await.map_err(db_err)?;
        Ok(count > 0)
    }

    async fn has_variant_inventory(&self, locator: &InventoryLocator) -> AppResult<bool> {
        let count = models::inventory_item::Entity::find()
            .filter(models::inventory_item::Column::StoreId.eq(locator.store_id))
            .filter(models::inventory_item::Column::ProductId.eq(locator.product_id))
            .filter(models::inventory_item::Column::ProductVariantId.is_not_null())
            .count(self.0.as_ref())
            .await
            .map_err(db_err)?;
        Ok(count > 0)
    }

    async fn has_product_level_inventory(&self, locator: &InventoryLocator) -> AppResult<bool> {
        let count = models::inventory_item::Entity::find()
            .filter(models::inventory_item::Column::StoreId.eq(locator.store_id))
            .filter(models::inventory_item::Column::ProductId.eq(locator.product_id))
            .filter(models::inventory_item::Column::ProductVariantId.is_null())
            .count(self.0.as_ref())
            .await
            .map_err(db_err)?;
        Ok(count > 0)
    }

    async fn list_paged(&self, page: PageRequest) -> AppResult<Page<InventoryItem>> {
        let paginator = models::inventory_item::Entity::find()
            .order_by(models::inventory_item::Column::CreatedAt, Order::Desc)
            .paginate(self.0.as_ref(), u64::from(page.per_page()));
        let total = paginator.num_items().await.map_err(db_err)?;
        let rows = paginator
            .fetch_page(u64::from(page.page() - 1))
            .await
            .map_err(db_err)?;
        Ok(Page::new(
            rows.into_iter().map(model_to_inventory_item).collect(),
            total,
            page.page(),
            page.per_page(),
        ))
    }

    async fn list_by_store(&self, store_id: i32, page: PageRequest) -> AppResult<Page<InventoryItem>> {
        let paginator = models::inventory_item::Entity::find()
            .filter(models::inventory_item::Column::StoreId.eq(store_id))
            .order_by(models::inventory_item::Column::CreatedAt, Order::Desc)
            .paginate(self.0.as_ref(), u64::from(page.per_page()));
        let total = paginator.num_items().await.map_err(db_err)?;
        let rows = paginator
            .fetch_page(u64::from(page.page() - 1))
            .await
            .map_err(db_err)?;
        Ok(Page::new(
            rows.into_iter().map(model_to_inventory_item).collect(),
            total,
            page.page(),
            page.per_page(),
        ))
    }

    async fn list_by_product(&self, product_id: Uuid) -> AppResult<Vec<InventoryItem>> {
        let rows = models::inventory_item::Entity::find()
            .filter(models::inventory_item::Column::ProductId.eq(product_id))
            .order_by(models::inventory_item::Column::CreatedAt, Order::Desc)
            .all(self.0.as_ref())
            .await
            .map_err(db_err)?;
        Ok(rows.into_iter().map(model_to_inventory_item).collect())
    }

    async fn list_low_stock(&self, store_id: Option<i32>, page: PageRequest) -> AppResult<Page<InventoryItem>> {
        let mut query = models::inventory_item::Entity::find().filter(
            Condition::all().add(
                sea_orm::sea_query::Expr::col(models::inventory_item::Column::Quantity).gt(0),
            ),
        );
        if let Some(store_id) = store_id {
            query = query.filter(models::inventory_item::Column::StoreId.eq(store_id));
        }
        let rows = query
            .order_by(models::inventory_item::Column::Quantity, Order::Asc)
            .all(self.0.as_ref())
            .await
            .map_err(db_err)?;
        let filtered: Vec<InventoryItem> = rows
            .into_iter()
            .map(model_to_inventory_item)
            .filter(|item| item.is_low_stock())
            .collect();
        let total = filtered.len() as u64;
        let start = page.offset() as usize;
        let items = filtered
            .into_iter()
            .skip(start)
            .take(page.per_page() as usize)
            .collect();
        Ok(Page::new(items, total, page.page(), page.per_page()))
    }

    async fn list_out_of_stock(&self, store_id: Option<i32>, page: PageRequest) -> AppResult<Page<InventoryItem>> {
        let mut query = models::inventory_item::Entity::find()
            .filter(models::inventory_item::Column::Quantity.eq(0));
        if let Some(store_id) = store_id {
            query = query.filter(models::inventory_item::Column::StoreId.eq(store_id));
        }
        let paginator = query
            .order_by(models::inventory_item::Column::CreatedAt, Order::Desc)
            .paginate(self.0.as_ref(), u64::from(page.per_page()));
        let total = paginator.num_items().await.map_err(db_err)?;
        let rows = paginator
            .fetch_page(u64::from(page.page() - 1))
            .await
            .map_err(db_err)?;
        Ok(Page::new(
            rows.into_iter().map(model_to_inventory_item).collect(),
            total,
            page.page(),
            page.per_page(),
        ))
    }
}

pub struct PgStockMovementRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl StockMovementRepository for PgStockMovementRepository {
    async fn find_by_id(&self, id: StockMovementId) -> AppResult<Option<StockMovement>> {
        let row = models::stock_movement::Entity::find_by_id(id.as_uuid())
            .one(self.0.as_ref())
            .await
            .map_err(db_err)?;
        Ok(row.map(model_to_stock_movement))
    }

    async fn save(&self, movement: &StockMovement) -> AppResult<()> {
        let active_model = models::stock_movement::ActiveModel {
            id: Set(movement.id.as_uuid()),
            product_id: Set(movement.locator.product_id),
            product_variant_id: Set(movement.locator.product_variant_id),
            store_id: Set(movement.locator.store_id),
            quantity_change: Set(movement.quantity_change),
            movement_type: Set(movement.movement_type.as_str().to_owned()),
            movement_date: Set(from_utc(movement.movement_date)),
            employee_id: Set(movement.employee_id),
            reference_id: Set(movement.reference_id.clone()),
            notes: Set(movement.notes.clone()),
            created_at: Set(from_utc(movement.created_at)),
            updated_at: Set(from_utc(movement.updated_at)),
        };
        active_model.insert(self.0.as_ref()).await.map_err(db_err)?;
        Ok(())
    }

    async fn list(
        &self,
        filter: StockMovementFilter,
        page: PageRequest,
    ) -> AppResult<Page<StockMovement>> {
        let mut query = models::stock_movement::Entity::find();
        if let Some(product_id) = filter.product_id {
            query = query.filter(models::stock_movement::Column::ProductId.eq(product_id));
        }
        if let Some(store_id) = filter.store_id {
            query = query.filter(models::stock_movement::Column::StoreId.eq(store_id));
        }
        if let Some(from_date) = filter.from_date {
            query = query.filter(models::stock_movement::Column::MovementDate.gte(from_utc(from_date)));
        }
        if let Some(to_date) = filter.to_date {
            query = query.filter(models::stock_movement::Column::MovementDate.lte(from_utc(to_date)));
        }
        if let Some(movement_type) = filter.movement_type {
            query = query.filter(
                models::stock_movement::Column::MovementType.eq(movement_type.as_str()),
            );
        }

        let paginator = query
            .order_by(models::stock_movement::Column::MovementDate, Order::Desc)
            .paginate(self.0.as_ref(), u64::from(page.per_page()));
        let total = paginator.num_items().await.map_err(db_err)?;
        let rows = paginator
            .fetch_page(u64::from(page.page() - 1))
            .await
            .map_err(db_err)?;
        Ok(Page::new(
            rows.into_iter().map(model_to_stock_movement).collect(),
            total,
            page.page(),
            page.per_page(),
        ))
    }
}