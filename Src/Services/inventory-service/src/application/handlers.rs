use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ddd_application::{
    register_command_handler, register_query_handler, CommandHandler, QueryHandler,
};
use ddd_shared_kernel::{AppError, AppResult, Page, PageRequest};

use super::commands::{
    AdjustInventoryQuantity, CreateInventoryItem, CreateStockMovement, ReleaseStock,
    ReserveStock, RestockInventory, UpdateInventoryQuantity, UpdateReorderLevel,
};
use super::deps::AppDeps;
use super::queries::{
    GetInventoryByProduct, GetInventoryByStore, GetInventoryItem, GetLowStockItems,
    GetOutOfStockItems, GetStockMovements, ListInventoryItems,
};
use crate::domain::entities::{InventoryItem, InventoryLocator, StockMovement};
use crate::domain::ids::{InventoryItemId, StockMovementId};
use crate::domain::repositories::{InventoryItemRepository, StockMovementFilter, StockMovementRepository};

fn parse_datetime(value: Option<String>) -> AppResult<Option<DateTime<Utc>>> {
    match value {
        Some(value) => DateTime::parse_from_rfc3339(&value)
            .map(|parsed| Some(parsed.with_timezone(&Utc)))
            .map_err(|_| AppError::validation("datetime", "must be RFC3339")),
        None => Ok(None),
    }
}

pub struct CreateInventoryItemHandler {
    repo: Arc<dyn InventoryItemRepository>,
}

#[async_trait]
impl CommandHandler<CreateInventoryItem> for CreateInventoryItemHandler {
    async fn handle(&self, cmd: CreateInventoryItem) -> AppResult<InventoryItemId> {
        let locator = InventoryLocator::new(cmd.product_id, cmd.product_variant_id, cmd.store_id)?;
        if self.repo.exists(locator.clone()).await? {
            return Err(AppError::conflict("inventory item already exists for this locator"));
        }

        if locator.product_variant_id.is_none() && self.repo.has_variant_inventory(&locator).await? {
            return Err(AppError::conflict(
                "product already has variant-level inventory for this store",
            ));
        }

        if locator.product_variant_id.is_some() && self.repo.has_product_level_inventory(&locator).await? {
            return Err(AppError::conflict(
                "product already has product-level inventory for this store",
            ));
        }

        let item = InventoryItem::create(
            InventoryItemId::new(),
            locator,
            cmd.initial_quantity,
            cmd.reorder_level,
        )?;
        let id = item.id;
        self.repo.save(&item).await?;
        Ok(id)
    }
}

register_command_handler!(CreateInventoryItem, AppDeps, |deps: &AppDeps| {
    CreateInventoryItemHandler {
        repo: deps.inventory_repo.clone(),
    }
});

pub struct UpdateInventoryQuantityHandler {
    repo: Arc<dyn InventoryItemRepository>,
}

#[async_trait]
impl CommandHandler<UpdateInventoryQuantity> for UpdateInventoryQuantityHandler {
    async fn handle(&self, cmd: UpdateInventoryQuantity) -> AppResult<()> {
        let mut item = self
            .repo
            .find_by_id(cmd.inventory_item_id)
            .await?
            .ok_or_else(|| AppError::not_found("InventoryItem", cmd.inventory_item_id.to_string()))?;
        item.update_quantity(cmd.new_quantity)?;
        self.repo.save(&item).await
    }
}

register_command_handler!(UpdateInventoryQuantity, AppDeps, |deps: &AppDeps| {
    UpdateInventoryQuantityHandler {
        repo: deps.inventory_repo.clone(),
    }
});

pub struct AdjustInventoryQuantityHandler {
    repo: Arc<dyn InventoryItemRepository>,
}

#[async_trait]
impl CommandHandler<AdjustInventoryQuantity> for AdjustInventoryQuantityHandler {
    async fn handle(&self, cmd: AdjustInventoryQuantity) -> AppResult<()> {
        let mut item = self
            .repo
            .find_by_id(cmd.inventory_item_id)
            .await?
            .ok_or_else(|| AppError::not_found("InventoryItem", cmd.inventory_item_id.to_string()))?;
        item.adjust_quantity(cmd.delta)?;
        self.repo.save(&item).await
    }
}

register_command_handler!(AdjustInventoryQuantity, AppDeps, |deps: &AppDeps| {
    AdjustInventoryQuantityHandler {
        repo: deps.inventory_repo.clone(),
    }
});

pub struct ReserveStockHandler {
    repo: Arc<dyn InventoryItemRepository>,
}

#[async_trait]
impl CommandHandler<ReserveStock> for ReserveStockHandler {
    async fn handle(&self, cmd: ReserveStock) -> AppResult<()> {
        let mut item = self
            .repo
            .find_by_id(cmd.inventory_item_id)
            .await?
            .ok_or_else(|| AppError::not_found("InventoryItem", cmd.inventory_item_id.to_string()))?;
        item.reserve_stock(cmd.quantity)?;
        self.repo.save(&item).await
    }
}

register_command_handler!(ReserveStock, AppDeps, |deps: &AppDeps| {
    ReserveStockHandler {
        repo: deps.inventory_repo.clone(),
    }
});

pub struct ReleaseStockHandler {
    repo: Arc<dyn InventoryItemRepository>,
}

#[async_trait]
impl CommandHandler<ReleaseStock> for ReleaseStockHandler {
    async fn handle(&self, cmd: ReleaseStock) -> AppResult<()> {
        let mut item = self
            .repo
            .find_by_id(cmd.inventory_item_id)
            .await?
            .ok_or_else(|| AppError::not_found("InventoryItem", cmd.inventory_item_id.to_string()))?;
        item.release_stock(cmd.quantity)?;
        self.repo.save(&item).await
    }
}

register_command_handler!(ReleaseStock, AppDeps, |deps: &AppDeps| {
    ReleaseStockHandler {
        repo: deps.inventory_repo.clone(),
    }
});

pub struct RestockInventoryHandler {
    repo: Arc<dyn InventoryItemRepository>,
}

#[async_trait]
impl CommandHandler<RestockInventory> for RestockInventoryHandler {
    async fn handle(&self, cmd: RestockInventory) -> AppResult<()> {
        let mut item = self
            .repo
            .find_by_id(cmd.inventory_item_id)
            .await?
            .ok_or_else(|| AppError::not_found("InventoryItem", cmd.inventory_item_id.to_string()))?;
        item.record_restock(cmd.quantity_added)?;
        self.repo.save(&item).await
    }
}

register_command_handler!(RestockInventory, AppDeps, |deps: &AppDeps| {
    RestockInventoryHandler {
        repo: deps.inventory_repo.clone(),
    }
});

pub struct UpdateReorderLevelHandler {
    repo: Arc<dyn InventoryItemRepository>,
}

#[async_trait]
impl CommandHandler<UpdateReorderLevel> for UpdateReorderLevelHandler {
    async fn handle(&self, cmd: UpdateReorderLevel) -> AppResult<()> {
        let mut item = self
            .repo
            .find_by_id(cmd.inventory_item_id)
            .await?
            .ok_or_else(|| AppError::not_found("InventoryItem", cmd.inventory_item_id.to_string()))?;
        item.update_reorder_level(cmd.new_reorder_level)?;
        self.repo.save(&item).await
    }
}

register_command_handler!(UpdateReorderLevel, AppDeps, |deps: &AppDeps| {
    UpdateReorderLevelHandler {
        repo: deps.inventory_repo.clone(),
    }
});

pub struct CreateStockMovementHandler {
    repo: Arc<dyn StockMovementRepository>,
}

#[async_trait]
impl CommandHandler<CreateStockMovement> for CreateStockMovementHandler {
    async fn handle(&self, cmd: CreateStockMovement) -> AppResult<StockMovementId> {
        let movement = StockMovement::create(
            StockMovementId::new(),
            InventoryLocator::new(cmd.product_id, cmd.product_variant_id, cmd.store_id)?,
            cmd.quantity_change,
            cmd.movement_type,
            cmd.employee_id,
            cmd.reference_id,
            cmd.notes,
        )?;
        let id = movement.id;
        self.repo.save(&movement).await?;
        Ok(id)
    }
}

register_command_handler!(CreateStockMovement, AppDeps, |deps: &AppDeps| {
    CreateStockMovementHandler {
        repo: deps.stock_movement_repo.clone(),
    }
});

pub struct GetInventoryItemHandler {
    repo: Arc<dyn InventoryItemRepository>,
}

#[async_trait]
impl QueryHandler<GetInventoryItem> for GetInventoryItemHandler {
    async fn handle(&self, query: GetInventoryItem) -> AppResult<Option<InventoryItem>> {
        self.repo.find_by_id(query.inventory_item_id).await
    }
}

register_query_handler!(GetInventoryItem, AppDeps, |deps: &AppDeps| {
    GetInventoryItemHandler {
        repo: deps.inventory_repo.clone(),
    }
});

pub struct ListInventoryItemsHandler {
    repo: Arc<dyn InventoryItemRepository>,
}

#[async_trait]
impl QueryHandler<ListInventoryItems> for ListInventoryItemsHandler {
    async fn handle(&self, query: ListInventoryItems) -> AppResult<Page<InventoryItem>> {
        self.repo
            .list_paged(PageRequest::new(query.page, query.per_page))
            .await
    }
}

register_query_handler!(ListInventoryItems, AppDeps, |deps: &AppDeps| {
    ListInventoryItemsHandler {
        repo: deps.inventory_repo.clone(),
    }
});

pub struct GetInventoryByStoreHandler {
    repo: Arc<dyn InventoryItemRepository>,
}

#[async_trait]
impl QueryHandler<GetInventoryByStore> for GetInventoryByStoreHandler {
    async fn handle(&self, query: GetInventoryByStore) -> AppResult<Page<InventoryItem>> {
        self.repo
            .list_by_store(query.store_id, PageRequest::new(query.page, query.per_page))
            .await
    }
}

register_query_handler!(GetInventoryByStore, AppDeps, |deps: &AppDeps| {
    GetInventoryByStoreHandler {
        repo: deps.inventory_repo.clone(),
    }
});

pub struct GetInventoryByProductHandler {
    repo: Arc<dyn InventoryItemRepository>,
}

#[async_trait]
impl QueryHandler<GetInventoryByProduct> for GetInventoryByProductHandler {
    async fn handle(&self, query: GetInventoryByProduct) -> AppResult<Vec<InventoryItem>> {
        self.repo.list_by_product(query.product_id).await
    }
}

register_query_handler!(GetInventoryByProduct, AppDeps, |deps: &AppDeps| {
    GetInventoryByProductHandler {
        repo: deps.inventory_repo.clone(),
    }
});

pub struct GetLowStockItemsHandler {
    repo: Arc<dyn InventoryItemRepository>,
}

#[async_trait]
impl QueryHandler<GetLowStockItems> for GetLowStockItemsHandler {
    async fn handle(&self, query: GetLowStockItems) -> AppResult<Page<InventoryItem>> {
        self.repo
            .list_low_stock(query.store_id, PageRequest::new(query.page, query.per_page))
            .await
    }
}

register_query_handler!(GetLowStockItems, AppDeps, |deps: &AppDeps| {
    GetLowStockItemsHandler {
        repo: deps.inventory_repo.clone(),
    }
});

pub struct GetOutOfStockItemsHandler {
    repo: Arc<dyn InventoryItemRepository>,
}

#[async_trait]
impl QueryHandler<GetOutOfStockItems> for GetOutOfStockItemsHandler {
    async fn handle(&self, query: GetOutOfStockItems) -> AppResult<Page<InventoryItem>> {
        self.repo
            .list_out_of_stock(query.store_id, PageRequest::new(query.page, query.per_page))
            .await
    }
}

register_query_handler!(GetOutOfStockItems, AppDeps, |deps: &AppDeps| {
    GetOutOfStockItemsHandler {
        repo: deps.inventory_repo.clone(),
    }
});

pub struct GetStockMovementsHandler {
    repo: Arc<dyn StockMovementRepository>,
}

#[async_trait]
impl QueryHandler<GetStockMovements> for GetStockMovementsHandler {
    async fn handle(&self, query: GetStockMovements) -> AppResult<Page<StockMovement>> {
        self.repo
            .list(
                StockMovementFilter {
                    product_id: query.product_id,
                    store_id: query.store_id,
                    from_date: parse_datetime(query.from_date)?,
                    to_date: parse_datetime(query.to_date)?,
                    movement_type: query.movement_type,
                },
                PageRequest::new(query.page, query.per_page),
            )
            .await
    }
}

register_query_handler!(GetStockMovements, AppDeps, |deps: &AppDeps| {
    GetStockMovementsHandler {
        repo: deps.stock_movement_repo.clone(),
    }
});