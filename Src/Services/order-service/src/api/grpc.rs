//! gRPC server implementation for OrderService.

use std::sync::Arc;

use ddd_api::grpc::error::GrpcErrorExt;
use ddd_application::Mediator;
use ddd_shared_kernel::{AggregateRoot, AppError};
use tonic::{Request, Response, Status};

use crate::application::commands::{CancelOrder, ConfirmOrder, CreateOrder};
use crate::application::queries::{GetOrder, ListOrders};
use crate::domain::aggregate::Order;
use crate::domain::events::OrderId;
use crate::domain::value_objects::{Money, OrderItem};
use crate::proto::{
    order_service_server::{OrderService, OrderServiceServer},
    CancelOrderRequest, CancelOrderResponse, ConfirmOrderRequest, ConfirmOrderResponse,
    CreateOrderRequest, CreateOrderResponse, GetOrderRequest, GetOrderResponse, ListOrdersRequest,
    ListOrdersResponse, Order as ProtoOrder, OrderItem as ProtoOrderItem,
};

#[derive(Clone)]
pub struct OrderGrpcService {
    mediator: Arc<Mediator>,
}

impl OrderGrpcService {
    pub fn new(mediator: Arc<Mediator>) -> Self {
        Self { mediator }
    }

    pub fn into_server(self) -> OrderServiceServer<Self> {
        OrderServiceServer::new(self)
    }
}

fn to_proto(o: &Order) -> ProtoOrder {
    ProtoOrder {
        id: o.id().to_string(),
        customer_id: o.customer_id.clone(),
        items: o.items.iter().map(item_to_proto).collect(),
        total_amount: o.total_amount.to_f64(),
        status: o.status.to_string(),
        created_at: o.created_at.to_rfc3339(),
        updated_at: o.updated_at().to_rfc3339(),
        version: o.version(),
    }
}

fn item_to_proto(i: &OrderItem) -> ProtoOrderItem {
    ProtoOrderItem {
        sku: i.sku.clone(),
        quantity: i.quantity,
        unit_price: i.unit_price.to_f64(),
    }
}

fn item_from_proto(i: ProtoOrderItem) -> OrderItem {
    OrderItem::new(i.sku, i.quantity, Money::from_f64(i.unit_price))
}

fn parse_id(s: &str) -> Result<OrderId, AppError> {
    OrderId::parse_str(s).map_err(|_| AppError::validation("id", "must be a valid UUID"))
}

#[tonic::async_trait]
impl OrderService for OrderGrpcService {
    async fn create_order(
        &self,
        req: Request<CreateOrderRequest>,
    ) -> Result<Response<CreateOrderResponse>, Status> {
        let r = req.into_inner();
        let id = self
            .mediator
            .send(CreateOrder {
                customer_id: r.customer_id,
                items: r.items.into_iter().map(item_from_proto).collect(),
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CreateOrderResponse {
            id: id.to_string(),
        }))
    }

    async fn get_order(
        &self,
        req: Request<GetOrderRequest>,
    ) -> Result<Response<GetOrderResponse>, Status> {
        let id = parse_id(&req.into_inner().id).map_err(|e| e.to_grpc_status())?;
        let order = self
            .mediator
            .query(GetOrder { order_id: id })
            .await
            .map_err(|e| e.to_grpc_status())?
            .ok_or_else(|| AppError::not_found("Order", id.to_string()).to_grpc_status())?;
        Ok(Response::new(GetOrderResponse {
            order: Some(to_proto(&order)),
        }))
    }

    async fn list_orders(
        &self,
        req: Request<ListOrdersRequest>,
    ) -> Result<Response<ListOrdersResponse>, Status> {
        let r = req.into_inner();
        let page = if r.page == 0 { 1 } else { r.page };
        let per_page = if r.per_page == 0 { 20 } else { r.per_page };
        let result = self
            .mediator
            .query(ListOrders { page, per_page })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ListOrdersResponse {
            orders: result.items().iter().map(to_proto).collect(),
            total: result.total(),
            page: result.page(),
            per_page: result.per_page(),
        }))
    }

    async fn confirm_order(
        &self,
        req: Request<ConfirmOrderRequest>,
    ) -> Result<Response<ConfirmOrderResponse>, Status> {
        let id = parse_id(&req.into_inner().id).map_err(|e| e.to_grpc_status())?;
        self.mediator
            .send(ConfirmOrder { order_id: id })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ConfirmOrderResponse {}))
    }

    async fn cancel_order(
        &self,
        req: Request<CancelOrderRequest>,
    ) -> Result<Response<CancelOrderResponse>, Status> {
        let r = req.into_inner();
        let id = parse_id(&r.id).map_err(|e| e.to_grpc_status())?;
        self.mediator
            .send(CancelOrder {
                order_id: id,
                reason: r.reason,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CancelOrderResponse {}))
    }
}
