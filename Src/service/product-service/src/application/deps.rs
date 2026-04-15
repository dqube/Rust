use std::sync::Arc;

use crate::application::ports::StoragePort;
use crate::domain::repository::ProductRepository;

#[derive(Clone)]
pub struct AppDeps {
    pub product_repo: Arc<dyn ProductRepository>,
    pub storage: Arc<dyn StoragePort>,
}
