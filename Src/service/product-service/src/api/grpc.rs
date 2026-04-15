//! gRPC server implementation for ProductService.

use std::sync::Arc;

use ddd_api::grpc::error::GrpcErrorExt;
use ddd_application::Mediator;
use ddd_shared_kernel::{AggregateRoot, AppError};
use tonic::{Request, Response, Status};

use crate::application::commands::{
    ConfirmImageUpload, CreateProduct, DeactivateProduct, RequestImageUploadUrl, UpdateStock,
};
use crate::application::queries::{GetProduct, ListProducts};
use crate::domain::aggregate::Product;
use crate::domain::events::ProductId;
use crate::proto::{
    product_service_server::{ProductService, ProductServiceServer},
    ConfirmImageUploadRequest, ConfirmImageUploadResponse, CreateProductRequest,
    CreateProductResponse, DeactivateProductRequest, DeactivateProductResponse, GetProductRequest,
    GetProductResponse, ListProductsRequest, ListProductsResponse, Product as ProtoProduct,
    RequestImageUploadUrlRequest, RequestImageUploadUrlResponse, UpdateStockRequest,
    UpdateStockResponse,
};

#[derive(Clone)]
pub struct ProductGrpcService {
    mediator: Arc<Mediator>,
}

impl ProductGrpcService {
    pub fn new(mediator: Arc<Mediator>) -> Self {
        Self { mediator }
    }

    pub fn into_server(self) -> ProductServiceServer<Self> {
        ProductServiceServer::new(self)
    }
}

fn to_proto(p: &Product) -> ProtoProduct {
    ProtoProduct {
        id: p.id().to_string(),
        sku: p.sku.clone(),
        name: p.name.clone(),
        description: p.description.clone(),
        price: p.price_cents as f64 / 100.0,
        stock: p.stock,
        active: p.active,
        created_at: p.created_at.to_rfc3339(),
        updated_at: p.updated_at().to_rfc3339(),
        version: p.version(),
        image_url: p.image_url.clone().unwrap_or_default(),
    }
}

fn parse_id(s: &str) -> Result<ProductId, AppError> {
    ProductId::parse_str(s).map_err(|_| AppError::validation("id", "must be a valid UUID"))
}

#[tonic::async_trait]
impl ProductService for ProductGrpcService {
    async fn create_product(
        &self,
        req: Request<CreateProductRequest>,
    ) -> Result<Response<CreateProductResponse>, Status> {
        let r = req.into_inner();
        let id = self
            .mediator
            .send(CreateProduct {
                sku: r.sku,
                name: r.name,
                description: r.description,
                price_cents: (r.price * 100.0).round() as i64,
                stock: r.stock,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CreateProductResponse {
            id: id.to_string(),
        }))
    }

    async fn get_product(
        &self,
        req: Request<GetProductRequest>,
    ) -> Result<Response<GetProductResponse>, Status> {
        let id = parse_id(&req.into_inner().id).map_err(|e| e.to_grpc_status())?;
        let product = self
            .mediator
            .query(GetProduct { product_id: id })
            .await
            .map_err(|e| e.to_grpc_status())?
            .ok_or_else(|| AppError::not_found("Product", id.to_string()).to_grpc_status())?;
        Ok(Response::new(GetProductResponse {
            product: Some(to_proto(&product)),
        }))
    }

    async fn list_products(
        &self,
        req: Request<ListProductsRequest>,
    ) -> Result<Response<ListProductsResponse>, Status> {
        let r = req.into_inner();
        let page = if r.page == 0 { 1 } else { r.page };
        let per_page = if r.per_page == 0 { 20 } else { r.per_page };
        let result = self
            .mediator
            .query(ListProducts { page, per_page })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ListProductsResponse {
            products: result.items().iter().map(to_proto).collect(),
            total: result.total(),
            page: result.page(),
            per_page: result.per_page(),
        }))
    }

    async fn update_stock(
        &self,
        req: Request<UpdateStockRequest>,
    ) -> Result<Response<UpdateStockResponse>, Status> {
        let r = req.into_inner();
        let id = parse_id(&r.id).map_err(|e| e.to_grpc_status())?;
        self.mediator
            .send(UpdateStock {
                product_id: id,
                stock: r.stock,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(UpdateStockResponse {}))
    }

    async fn deactivate_product(
        &self,
        req: Request<DeactivateProductRequest>,
    ) -> Result<Response<DeactivateProductResponse>, Status> {
        let id = parse_id(&req.into_inner().id).map_err(|e| e.to_grpc_status())?;
        self.mediator
            .send(DeactivateProduct { product_id: id })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(DeactivateProductResponse {}))
    }

    async fn request_image_upload_url(
        &self,
        req: Request<RequestImageUploadUrlRequest>,
    ) -> Result<Response<RequestImageUploadUrlResponse>, Status> {
        let r = req.into_inner();
        let product_id = parse_id(&r.product_id).map_err(|e| e.to_grpc_status())?;
        let (upload_url, expires_in_secs) = self
            .mediator
            .send(RequestImageUploadUrl {
                product_id,
                filename: r.filename,
                content_type: r.content_type,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(RequestImageUploadUrlResponse {
            upload_url,
            expires_in_secs,
        }))
    }

    async fn confirm_image_upload(
        &self,
        req: Request<ConfirmImageUploadRequest>,
    ) -> Result<Response<ConfirmImageUploadResponse>, Status> {
        let r = req.into_inner();
        let product_id = parse_id(&r.product_id).map_err(|e| e.to_grpc_status())?;
        self.mediator
            .send(ConfirmImageUpload {
                product_id,
                image_url: r.image_url,
            })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(ConfirmImageUploadResponse {}))
    }
}
