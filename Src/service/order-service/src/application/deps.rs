//! Shared dependency container for all handlers.

use std::sync::Arc;

use crate::domain::repository::OrderRepository;

/// Application-level dependency container, injected into handlers via
/// `Mediator::from_inventory`.
#[derive(Clone)]
pub struct AppDeps {
    pub order_repo: Arc<dyn OrderRepository>,
}
