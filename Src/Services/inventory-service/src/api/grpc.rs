use std::sync::Arc;

use ddd_api::grpc::error::GrpcErrorExt;
use ddd_application::Mediator;
use ddd_shared_kernel::AppError;
use std::str::FromStr;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::application::commands::{
    AdjustInventoryQuantity, CreateInventoryItem, CreateStockMovement, ReleaseStock,
    ReserveStock, RestockInventory, UpdateInventoryQuantity, UpdateReorderLevel,
};
use crate::application::queries::{
    GetInventoryByProduct, GetInventoryByStore, GetInventoryItem, GetLowStockItems,
    GetOutOfStockItems, GetStockMovements, ListInventoryItems,
};
use crate::domain::entities::{InventoryItem, StockMovement};
use crate::domain::enums::MovementType;
use crate::domain::ids::InventoryItemId;
use crate::proto::{
    inventory_grpc_server::{InventoryGrpc, InventoryGrpcServer},
    AdjustInventoryQuantityRequest, AdjustInventoryQuantityResponse, CreateInventoryItemRequest,
    CreateInventoryItemResponse, CreateStockMovementRequest, CreateStockMovementResponse,
    GetInventoryByProductRequest, GetInventoryByProductResponse, GetInventoryByStoreRequest,
    GetInventoryItemRequest, GetInventoryItemResponse, GetLowStockItemsRequest,
    GetOutOfStockItemsRequest, InventoryItemInfo, ListInventoryItemsRequest,
    ListInventoryItemsResponse, ListStockMovementsRequest, ListStockMovementsResponse,
    ReleaseStockRequest, ReleaseStockResponse, ReserveStockRequest, ReserveStockResponse,
    RestockInventoryRequest, RestockInventoryResponse, StockMovementInfo,
    UpdateInventoryQuantityRequest, UpdateInventoryQuantityResponse,
    UpdateReorderLevelRequest, UpdateReorderLevelResponse,
};

#[derive(Clone)]
pub struct InventoryGrpcService {
    mediator: Arc<Mediator>,
}

impl InventoryGrpcService {
    pub fn new(mediator: Arc<Mediator>) -> Self {
        Self { mediator }
    }

    pub fn into_server(self) -> InventoryGrpcServer<Self> {
        InventoryGrpcServer::new(self)
    }
}

fn parse_inventory_item_id(value: &str) -> Result<InventoryItemId, AppError> {
    InventoryItemId::parse_str(value)
        .map_err(|_| AppError::validation("inventory_item_id", "must be a valid UUID"))
}

fn parse_store_id(value: &str) -> Result<i32, AppError> {
    let store_id = value
        .parse::<i32>()
        .map_err(|_| AppError::validation("store_id", "must be a valid integer"))?;
    if store_id <= 0 {
        return Err(AppError::validation(
            "store_id",
            "must be a positive integer",
        ));
    }
    Ok(store_id)
}

fn parse_optional_store_id(value: &str) -> Result<Option<i32>, AppError> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    parse_store_id(value).map(Some)
}

fn parse_uuid(field: &str, value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| AppError::validation(field, "must be a valid UUID"))
}

fn parse_optional_uuid(field: &str, value: &str) -> Result<Option<Uuid>, AppError> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    parse_uuid(field, value).map(Some)
}

fn parse_movement_type(value: &str) -> Result<MovementType, AppError> {
    MovementType::from_str(value)
    .map_err(|_| AppError::validation("movement_type", "is not a supported movement"))
}

fn item_to_proto(item: &InventoryItem) -> InventoryItemInfo {
    InventoryItemInfo {
        inventory_item_id: item.id.to_string(),
        product_id: item.locator.product_id.to_string(),
        store_id: item.locator.store_id.to_string(),
        quantity: item.quantity,
        reserved_quantity: item.reserved_quantity,
        available_quantity: item.available_quantity(),
        reorder_level: item.reorder_level,
        is_low_stock: item.is_low_stock(),
        is_out_of_stock: item.is_out_of_stock(),
        last_restock_date: item
            .last_restock_date
            .map(|value| value.to_rfc3339())
            .unwrap_or_default(),
        created_at: item.created_at.to_rfc3339(),
        updated_at: item.updated_at.to_rfc3339(),
        product_variant_id: item
            .locator
            .product_variant_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
    }
}

fn movement_to_proto(movement: &StockMovement) -> StockMovementInfo {
    StockMovementInfo {
        stock_movement_id: movement.id.to_string(),
        product_id: movement.locator.product_id.to_string(),
        store_id: movement.locator.store_id.to_string(),
        quantity_change: movement.quantity_change,
        movement_type: movement.movement_type.as_str().to_owned(),
        movement_date: movement.movement_date.to_rfc3339(),
        employee_id: movement
            .employee_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        reference_id: movement.reference_id.clone().unwrap_or_default(),
        notes: movement.notes.clone().unwrap_or_default(),
        created_at: movement.created_at.to_rfc3339(),
    }
}

fn paged_items_response(page: &ddd_shared_kernel::Page<InventoryItem>) -> ListInventoryItemsResponse {
    ListInventoryItemsResponse {
        items: page.items().iter().map(item_to_proto).collect(),
        total_count: page.total() as i32,
        page: page.page() as i32,
        page_size: page.per_page() as i32,
        total_pages: page.total_pages() as i32,
    }
}

#[tonic::async_trait]
impl InventoryGrpc for InventoryGrpcService {
    async fn create_inventory_item(
        &self,
        request: Request<CreateInventoryItemRequest>,
    ) -> Result<Response<CreateInventoryItemResponse>, Status> {
        let request = request.into_inner();
        let result = self
            .mediator
            .send(CreateInventoryItem {
                product_id: parse_uuid("product_id", &request.product_id).map_err(|e| e.to_grpc_status())?,
                product_variant_id: parse_optional_uuid("product_variant_id", &request.product_variant_id)
                    .map_err(|e| e.to_grpc_status())?,
                store_id: parse_store_id(&request.store_id).map_err(|e| e.to_grpc_status())?,
                initial_quantity: request.initial_quantity,
                reorder_level: request.reorder_level,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CreateInventoryItemResponse {
            inventory_item_id: result.to_string(),
        }))
    }

    async fn get_inventory_item(
        &self,
        request: Request<GetInventoryItemRequest>,
    ) -> Result<Response<GetInventoryItemResponse>, Status> {
        let request = request.into_inner();
        let item = self
            .mediator
            .query(GetInventoryItem {
                inventory_item_id: parse_inventory_item_id(&request.inventory_item_id)
                    .map_err(|e| e.to_grpc_status())?,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;

        Ok(Response::new(GetInventoryItemResponse {
            found: item.is_some(),
            item: item.as_ref().map(item_to_proto),
        }))
    }

    async fn list_inventory_items(
        &self,
        request: Request<ListInventoryItemsRequest>,
    ) -> Result<Response<ListInventoryItemsResponse>, Status> {
        let request = request.into_inner();
        let page = self
            .mediator
            .query(ListInventoryItems {
                page: request.page as u32,
                per_page: request.page_size as u32,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(paged_items_response(&page)))
    }

    async fn get_inventory_by_store(
        &self,
        request: Request<GetInventoryByStoreRequest>,
    ) -> Result<Response<ListInventoryItemsResponse>, Status> {
        let request = request.into_inner();
        let page = self
            .mediator
            .query(GetInventoryByStore {
                store_id: parse_store_id(&request.store_id).map_err(|e| e.to_grpc_status())?,
                page: request.page as u32,
                per_page: request.page_size as u32,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(paged_items_response(&page)))
    }

    async fn get_inventory_by_product(
        &self,
        request: Request<GetInventoryByProductRequest>,
    ) -> Result<Response<GetInventoryByProductResponse>, Status> {
        let request = request.into_inner();
        let items = self
            .mediator
            .query(GetInventoryByProduct {
                product_id: parse_uuid("product_id", &request.product_id).map_err(|e| e.to_grpc_status())?,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(GetInventoryByProductResponse {
            items: items.iter().map(item_to_proto).collect(),
        }))
    }

    async fn update_inventory_quantity(
        &self,
        request: Request<UpdateInventoryQuantityRequest>,
    ) -> Result<Response<UpdateInventoryQuantityResponse>, Status> {
        let request = request.into_inner();
        self.mediator
            .send(UpdateInventoryQuantity {
                inventory_item_id: parse_inventory_item_id(&request.inventory_item_id)
                    .map_err(|e| e.to_grpc_status())?,
                new_quantity: request.new_quantity,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(UpdateInventoryQuantityResponse {}))
    }

    async fn adjust_inventory_quantity(
        &self,
        request: Request<AdjustInventoryQuantityRequest>,
    ) -> Result<Response<AdjustInventoryQuantityResponse>, Status> {
        let request = request.into_inner();
        self.mediator
            .send(AdjustInventoryQuantity {
                inventory_item_id: parse_inventory_item_id(&request.inventory_item_id)
                    .map_err(|e| e.to_grpc_status())?,
                delta: request.delta,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(AdjustInventoryQuantityResponse {}))
    }

    async fn reserve_stock(
        &self,
        request: Request<ReserveStockRequest>,
    ) -> Result<Response<ReserveStockResponse>, Status> {
        let request = request.into_inner();
        self.mediator
            .send(ReserveStock {
                inventory_item_id: parse_inventory_item_id(&request.inventory_item_id)
                    .map_err(|e| e.to_grpc_status())?,
                quantity: request.quantity,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ReserveStockResponse {}))
    }

    async fn release_stock(
        &self,
        request: Request<ReleaseStockRequest>,
    ) -> Result<Response<ReleaseStockResponse>, Status> {
        let request = request.into_inner();
        self.mediator
            .send(ReleaseStock {
                inventory_item_id: parse_inventory_item_id(&request.inventory_item_id)
                    .map_err(|e| e.to_grpc_status())?,
                quantity: request.quantity,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ReleaseStockResponse {}))
    }

    async fn restock_inventory(
        &self,
        request: Request<RestockInventoryRequest>,
    ) -> Result<Response<RestockInventoryResponse>, Status> {
        let request = request.into_inner();
        self.mediator
            .send(RestockInventory {
                inventory_item_id: parse_inventory_item_id(&request.inventory_item_id)
                    .map_err(|e| e.to_grpc_status())?,
                quantity_added: request.quantity_added,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(RestockInventoryResponse {}))
    }

    async fn create_stock_movement(
        &self,
        request: Request<CreateStockMovementRequest>,
    ) -> Result<Response<CreateStockMovementResponse>, Status> {
        let request = request.into_inner();
        let result = self
            .mediator
            .send(CreateStockMovement {
                product_id: parse_uuid("product_id", &request.product_id).map_err(|e| e.to_grpc_status())?,
                product_variant_id: parse_optional_uuid("product_variant_id", &request.product_variant_id)
                    .map_err(|e| e.to_grpc_status())?,
                store_id: parse_store_id(&request.store_id).map_err(|e| e.to_grpc_status())?,
                quantity_change: request.quantity_change,
                movement_type: parse_movement_type(&request.movement_type).map_err(|e| e.to_grpc_status())?,
                employee_id: parse_optional_uuid("employee_id", &request.employee_id)
                    .map_err(|e| e.to_grpc_status())?,
                reference_id: (!request.reference_id.trim().is_empty()).then_some(request.reference_id),
                notes: (!request.notes.trim().is_empty()).then_some(request.notes),
            })
            .await
            .map_err(|e| e.to_grpc_status())?;

        Ok(Response::new(CreateStockMovementResponse {
            stock_movement_id: result.to_string(),
        }))
    }

    async fn list_stock_movements(
        &self,
        request: Request<ListStockMovementsRequest>,
    ) -> Result<Response<ListStockMovementsResponse>, Status> {
        let request = request.into_inner();
        let page = self
            .mediator
            .query(GetStockMovements {
                product_id: parse_optional_uuid("product_id", &request.product_id)
                    .map_err(|e| e.to_grpc_status())?,
                store_id: parse_optional_store_id(&request.store_id).map_err(|e| e.to_grpc_status())?,
                from_date: (!request.from_date.trim().is_empty()).then_some(request.from_date),
                to_date: (!request.to_date.trim().is_empty()).then_some(request.to_date),
                movement_type: if request.movement_type.trim().is_empty() {
                    None
                } else {
                    Some(parse_movement_type(&request.movement_type).map_err(|e| e.to_grpc_status())?)
                },
                page: request.page as u32,
                per_page: request.page_size as u32,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ListStockMovementsResponse {
            items: page.items().iter().map(movement_to_proto).collect(),
            total_count: page.total() as i32,
            page: page.page() as i32,
            page_size: page.per_page() as i32,
            total_pages: page.total_pages() as i32,
        }))
    }

    async fn get_low_stock_items(
        &self,
        request: Request<GetLowStockItemsRequest>,
    ) -> Result<Response<ListInventoryItemsResponse>, Status> {
        let request = request.into_inner();
        let page = self
            .mediator
            .query(GetLowStockItems {
                store_id: parse_optional_store_id(&request.store_id).map_err(|e| e.to_grpc_status())?,
                page: request.page as u32,
                per_page: request.page_size as u32,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(paged_items_response(&page)))
    }

    async fn get_out_of_stock_items(
        &self,
        request: Request<GetOutOfStockItemsRequest>,
    ) -> Result<Response<ListInventoryItemsResponse>, Status> {
        let request = request.into_inner();
        let page = self
            .mediator
            .query(GetOutOfStockItems {
                store_id: parse_optional_store_id(&request.store_id).map_err(|e| e.to_grpc_status())?,
                page: request.page as u32,
                per_page: request.page_size as u32,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(paged_items_response(&page)))
    }

    async fn update_reorder_level(
        &self,
        request: Request<UpdateReorderLevelRequest>,
    ) -> Result<Response<UpdateReorderLevelResponse>, Status> {
        let request = request.into_inner();
        self.mediator
            .send(UpdateReorderLevel {
                inventory_item_id: parse_inventory_item_id(&request.inventory_item_id)
                    .map_err(|e| e.to_grpc_status())?,
                new_reorder_level: request.new_reorder_level,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(UpdateReorderLevelResponse {}))
    }
}