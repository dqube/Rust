use std::sync::Arc;
use ddd_bff::clients::GrpcClientPool;
use ddd_shared_kernel::jwt::{JwtValidator, StandardClaims};
use ddd_shared_kernel::Cache;
use crate::application::config::AdminBffConfig;
use crate::infrastructure::clients::auth::AuthClient;
use crate::infrastructure::clients::customer::CustomerClient;
use crate::infrastructure::clients::employee::EmployeeClient;
use crate::infrastructure::clients::order::OrderClient;
use crate::infrastructure::clients::product::ProductClient;
use crate::infrastructure::clients::shared::SharedClient;
use crate::infrastructure::clients::supplier::SupplierClient;

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
    pub auth_client: Arc<AuthClient>,
    pub customer_client: Arc<CustomerClient>,
    pub employee_client: Arc<EmployeeClient>,
    pub supplier_client: Arc<SupplierClient>,
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
        let auth_channel = pool
            .channel("auth")
            .expect("auth channel registered");
        let customer_channel = pool
            .channel("customer")
            .expect("customer channel registered");
        let employee_channel = pool
            .channel("employee")
            .expect("employee channel registered");
        let supplier_channel = pool
            .channel("supplier")
            .expect("supplier channel registered");

        Self {
            config: Arc::new(config),
            product_client: Arc::new(ProductClient::new(product_channel)),
            order_client: Arc::new(OrderClient::new(order_channel)),
            shared_client: Arc::new(SharedClient::new(shared_channel)),
            auth_client: Arc::new(AuthClient::new(auth_channel)),
            customer_client: Arc::new(CustomerClient::new(customer_channel)),
            employee_client: Arc::new(EmployeeClient::new(employee_channel)),
            supplier_client: Arc::new(SupplierClient::new(supplier_channel)),
            jwt_validator,
            pool: Arc::new(pool),
            cache,
        }
    }
}
