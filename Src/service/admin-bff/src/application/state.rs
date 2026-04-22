use std::sync::Arc;
use ddd_bff::clients::GrpcClientPool;
use ddd_shared_kernel::jwt::{JwtValidator, StandardClaims};
use ddd_shared_kernel::Cache;
use crate::application::config::AdminBffConfig;
use crate::infrastructure::clients::order::OrderClient;
use crate::infrastructure::clients::product::ProductClient;

/// Unified shared state for all handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AdminBffConfig>,
    pub product_client: Arc<ProductClient>,
    pub order_client: Arc<OrderClient>,
    pub jwt_validator: Option<Arc<JwtValidator<StandardClaims>>>,
    pub order_channel: tonic::transport::Channel,
    /// Optional read-through cache. `None` when `REDIS_URL` is unset.
    pub cache: Option<Arc<dyn Cache>>,
}

impl AppState {
    pub fn new(
        config: AdminBffConfig,
        pool: GrpcClientPool,
        jwt_validator: Option<Arc<JwtValidator<StandardClaims>>>,
        cache: Option<Arc<dyn Cache>>,
    ) -> Self {
        let product_channel = pool
            .channel("product")
            .expect("product channel registered");
        let order_channel = pool
            .channel("order")
            .expect("order channel registered");

        Self {
            config: Arc::new(config),
            product_client: Arc::new(ProductClient::new(product_channel)),
            order_client: Arc::new(OrderClient::new(order_channel.clone())),
            jwt_validator,
            order_channel, // Used for aggregation fan-out
            cache,
        }
    }
}
