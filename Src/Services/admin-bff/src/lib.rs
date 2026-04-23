//! Admin BFF — REST gateway proxying product-service via gRPC
//! and order-service via gRPC, with observability and metrics.
//!
//! Layered per the project's clean-architecture convention:
//!
//! - [`api`] — inbound REST adapter (router, OpenAPI, handlers).
//! - [`application`] — configuration and shared state.
//! - [`infrastructure`] — outbound gRPC clients to downstream services.

pub mod api;
pub mod application;
pub mod infrastructure;

pub mod proto {
    tonic::include_proto!("product.v1");
}

pub mod proto_order {
    tonic::include_proto!("order.v1");
}

pub mod proto_shared {
    tonic::include_proto!("shared.v1");
}

pub mod proto_auth {
    tonic::include_proto!("auth.v1");
}

pub mod proto_customer {
    tonic::include_proto!("customer.v1");
}

pub mod proto_employee {
    tonic::include_proto!("employee.v1");
}

pub mod proto_supplier {
    tonic::include_proto!("supplier.v1");
}

pub mod proto_catalog {
    tonic::include_proto!("catalog.v1");
}

pub mod proto_sales {
    tonic::include_proto!("sales.v1");
}
