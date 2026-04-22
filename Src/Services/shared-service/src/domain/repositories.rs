use async_trait::async_trait;
use ddd_shared_kernel::AppResult;

use super::entities::{City, Country, Currency, Pincode, State};
use super::ids::{CityCode, CountryCode, CurrencyCode, PincodeId, StateCode};

#[async_trait]
pub trait CurrencyRepository: Send + Sync {
    async fn find_by_id(&self, id: &CurrencyCode) -> AppResult<Option<Currency>>;
    async fn find_all(&self) -> AppResult<Vec<Currency>>;
    async fn exists(&self, id: &CurrencyCode) -> AppResult<bool>;
    async fn save(&self, currency: &Currency) -> AppResult<()>;
    async fn delete(&self, id: &CurrencyCode) -> AppResult<()>;
}

#[async_trait]
pub trait CountryRepository: Send + Sync {
    async fn find_by_id(&self, id: &CountryCode) -> AppResult<Option<Country>>;
    async fn find_all(&self) -> AppResult<Vec<Country>>;
    async fn find_by_currency(&self, currency_code: &CurrencyCode) -> AppResult<Vec<Country>>;
    async fn exists(&self, id: &CountryCode) -> AppResult<bool>;
    async fn save(&self, country: &Country) -> AppResult<()>;
    async fn delete(&self, id: &CountryCode) -> AppResult<()>;
}

#[async_trait]
pub trait StateRepository: Send + Sync {
    async fn find_by_id(&self, id: &StateCode) -> AppResult<Option<State>>;
    async fn find_all(&self) -> AppResult<Vec<State>>;
    async fn find_by_country(&self, country_code: &CountryCode) -> AppResult<Vec<State>>;
    async fn exists(&self, id: &StateCode) -> AppResult<bool>;
    async fn save(&self, state: &State) -> AppResult<()>;
    async fn delete(&self, id: &StateCode) -> AppResult<()>;
}

#[async_trait]
pub trait CityRepository: Send + Sync {
    async fn find_by_id(&self, id: &CityCode) -> AppResult<Option<City>>;
    async fn find_all(&self) -> AppResult<Vec<City>>;
    async fn find_by_state(&self, state_code: &StateCode) -> AppResult<Vec<City>>;
    async fn exists(&self, id: &CityCode) -> AppResult<bool>;
    async fn save(&self, city: &City) -> AppResult<()>;
    async fn delete(&self, id: &CityCode) -> AppResult<()>;
}

#[async_trait]
pub trait PincodeRepository: Send + Sync {
    async fn find_by_id(&self, id: &PincodeId) -> AppResult<Option<Pincode>>;
    async fn find_all(&self) -> AppResult<Vec<Pincode>>;
    async fn find_by_city(&self, city_code: &CityCode) -> AppResult<Vec<Pincode>>;
    async fn exists(&self, id: &PincodeId) -> AppResult<bool>;
    async fn save(&self, pincode: &Pincode) -> AppResult<()>;
    async fn delete(&self, id: &PincodeId) -> AppResult<()>;
}
