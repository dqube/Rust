use std::sync::Arc;

use crate::domain::repositories::{
    CityRepository, CountryRepository, CurrencyRepository, PincodeRepository, StateRepository,
};

/// Dependency container handed to inventory-registered handler factories.
#[derive(Clone)]
pub struct AppDeps {
    pub currency_repo: Arc<dyn CurrencyRepository>,
    pub country_repo: Arc<dyn CountryRepository>,
    pub state_repo: Arc<dyn StateRepository>,
    pub city_repo: Arc<dyn CityRepository>,
    pub pincode_repo: Arc<dyn PincodeRepository>,
}
