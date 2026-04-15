//! gRPC client connection pool and resilient client wrapper.

pub mod pool;
pub mod resilient_client;

pub use pool::GrpcClientPool;
pub use resilient_client::ResilientChannel;
