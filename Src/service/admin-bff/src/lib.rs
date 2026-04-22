//! Admin BFF — REST gateway proxying product-service via gRPC
//! and order-service via HTTP, with observability and metrics.

pub mod aggregation;
pub mod clients;
pub mod config;
pub mod handlers;
pub mod openapi;
pub mod openapi_routes;
pub mod router;
pub mod state;

pub mod proto {
    tonic::include_proto!("product.v1");
}

pub mod proto_order {
    tonic::include_proto!("order.v1");
}
