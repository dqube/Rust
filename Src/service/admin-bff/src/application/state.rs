use std::sync::Arc;
use ddd_bff::clients::GrpcClientPool;
use ddd_shared_kernel::jwt::{JwtValidator, StandardClaims};
use ddd_shared_kernel::Cache;
use crate::application::config::AdminBffConfig;
use crate::infrastructure::clients::order::OrderClient;
use crate::infrastructure::clients::product::ProductClient;
use crate::infrastructure::clients::shared::SharedClient;

/// Unified shared state for all handlers.
///
/// The `pool` handle is retained so diagnostics (and future routes that
/// need to open additional streams) can inspect the [`ResilientChannel`]
/// metadata — timeout and max-concurrent — configured at startup.
///
/// [`ResilientChannel`]: ddd_bff::clients::ResilientChannel
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AdminBffConfig>,
    pub product_client: Arc<ProductClient>,
    pub order_client: Arc<OrderClient>,
    pub shared_client: Arc<SharedClient>,
    pub jwt_validator: Option<Arc<JwtValidator<StandardClaims>>>,
    pub pool: Arc<GrpcClientPool>,
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
        let shared_channel = pool
            .channel("shared")
            .expect("shared channel registered");

        Self {
            config: Arc::new(config),
            product_client: Arc::new(ProductClient::new(product_channel)),
            order_client: Arc::new(OrderClient::new(order_channel)),
            shared_client: Arc::new(SharedClient::new(shared_channel)),
            jwt_validator,
            pool: Arc::new(pool),
            cache,
        }
    }
}
