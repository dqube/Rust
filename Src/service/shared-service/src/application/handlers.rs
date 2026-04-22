//! Command + query handlers for shared-service.
//!
//! All handlers are registered via the [`inventory`] crate so they are
//! discovered at link time by [`ddd_application::Mediator::from_inventory`].

use std::sync::Arc;

use async_trait::async_trait;
use ddd_application::{
    register_command_handler, register_query_handler, CommandHandler, QueryHandler,
};
use ddd_shared_kernel::{AppError, AppResult};

use super::commands::*;
use super::deps::AppDeps;
use super::queries::*;
use crate::domain::entities::{City, Country, Currency, Pincode, State};
use crate::domain::ids::{CityCode, CountryCode, CurrencyCode, PincodeId, StateCode};
use crate::domain::repositories::{
    CityRepository, CountryRepository, CurrencyRepository, PincodeRepository, StateRepository,
};

// ─── Currency ────────────────────────────────────────────────────────────────

pub struct CreateCurrencyHandler {
    repo: Arc<dyn CurrencyRepository>,
}
#[async_trait]
impl CommandHandler<CreateCurrency> for CreateCurrencyHandler {
    async fn handle(&self, cmd: CreateCurrency) -> AppResult<()> {
        let id = CurrencyCode::new(&cmd.code);
        if self.repo.exists(&id).await? {
            return Err(AppError::conflict(format!(
                "Currency '{}' already exists.",
                id
            )));
        }
        let c = Currency::create(id, cmd.name, cmd.symbol)?;
        self.repo.save(&c).await
    }
}
register_command_handler!(CreateCurrency, AppDeps, |d: &AppDeps| {
    CreateCurrencyHandler { repo: d.currency_repo.clone() }
});

pub struct UpdateCurrencyHandler { repo: Arc<dyn CurrencyRepository> }
#[async_trait]
impl CommandHandler<UpdateCurrency> for UpdateCurrencyHandler {
    async fn handle(&self, cmd: UpdateCurrency) -> AppResult<()> {
        let id = CurrencyCode::new(&cmd.code);
        let mut c = self.repo.find_by_id(&id).await?
            .ok_or_else(|| AppError::not_found("Currency", id.to_string()))?;
        c.update(cmd.name, cmd.symbol)?;
        self.repo.save(&c).await
    }
}
register_command_handler!(UpdateCurrency, AppDeps, |d: &AppDeps| {
    UpdateCurrencyHandler { repo: d.currency_repo.clone() }
});

pub struct DeleteCurrencyHandler { repo: Arc<dyn CurrencyRepository> }
#[async_trait]
impl CommandHandler<DeleteCurrency> for DeleteCurrencyHandler {
    async fn handle(&self, cmd: DeleteCurrency) -> AppResult<()> {
        let id = CurrencyCode::new(&cmd.code);
        if !self.repo.exists(&id).await? {
            return Err(AppError::not_found("Currency", id.to_string()));
        }
        self.repo.delete(&id).await
    }
}
register_command_handler!(DeleteCurrency, AppDeps, |d: &AppDeps| {
    DeleteCurrencyHandler { repo: d.currency_repo.clone() }
});

pub struct ActivateCurrencyHandler { repo: Arc<dyn CurrencyRepository> }
#[async_trait]
impl CommandHandler<ActivateCurrency> for ActivateCurrencyHandler {
    async fn handle(&self, cmd: ActivateCurrency) -> AppResult<()> {
        let id = CurrencyCode::new(&cmd.code);
        let mut c = self.repo.find_by_id(&id).await?
            .ok_or_else(|| AppError::not_found("Currency", id.to_string()))?;
        c.activate();
        self.repo.save(&c).await
    }
}
register_command_handler!(ActivateCurrency, AppDeps, |d: &AppDeps| {
    ActivateCurrencyHandler { repo: d.currency_repo.clone() }
});

pub struct DeactivateCurrencyHandler { repo: Arc<dyn CurrencyRepository> }
#[async_trait]
impl CommandHandler<DeactivateCurrency> for DeactivateCurrencyHandler {
    async fn handle(&self, cmd: DeactivateCurrency) -> AppResult<()> {
        let id = CurrencyCode::new(&cmd.code);
        let mut c = self.repo.find_by_id(&id).await?
            .ok_or_else(|| AppError::not_found("Currency", id.to_string()))?;
        c.deactivate();
        self.repo.save(&c).await
    }
}
register_command_handler!(DeactivateCurrency, AppDeps, |d: &AppDeps| {
    DeactivateCurrencyHandler { repo: d.currency_repo.clone() }
});

pub struct GetCurrencyByCodeHandler { repo: Arc<dyn CurrencyRepository> }
#[async_trait]
impl QueryHandler<GetCurrencyByCode> for GetCurrencyByCodeHandler {
    async fn handle(&self, q: GetCurrencyByCode) -> AppResult<Option<Currency>> {
        self.repo.find_by_id(&CurrencyCode::new(&q.code)).await
    }
}
register_query_handler!(GetCurrencyByCode, AppDeps, |d: &AppDeps| {
    GetCurrencyByCodeHandler { repo: d.currency_repo.clone() }
});

pub struct GetAllCurrenciesHandler { repo: Arc<dyn CurrencyRepository> }
#[async_trait]
impl QueryHandler<GetAllCurrencies> for GetAllCurrenciesHandler {
    async fn handle(&self, _q: GetAllCurrencies) -> AppResult<Vec<Currency>> {
        self.repo.find_all().await
    }
}
register_query_handler!(GetAllCurrencies, AppDeps, |d: &AppDeps| {
    GetAllCurrenciesHandler { repo: d.currency_repo.clone() }
});

// ─── Country ─────────────────────────────────────────────────────────────────

pub struct CreateCountryHandler {
    country_repo: Arc<dyn CountryRepository>,
    currency_repo: Arc<dyn CurrencyRepository>,
}
#[async_trait]
impl CommandHandler<CreateCountry> for CreateCountryHandler {
    async fn handle(&self, cmd: CreateCountry) -> AppResult<()> {
        let id = CountryCode::new(&cmd.code);
        if self.country_repo.exists(&id).await? {
            return Err(AppError::conflict(format!("Country '{}' already exists.", id)));
        }
        let cur = CurrencyCode::new(&cmd.currency_code);
        if !self.currency_repo.exists(&cur).await? {
            return Err(AppError::not_found("Currency", cur.to_string()));
        }
        let c = Country::create(id, cmd.name, cur)?;
        self.country_repo.save(&c).await
    }
}
register_command_handler!(CreateCountry, AppDeps, |d: &AppDeps| {
    CreateCountryHandler {
        country_repo: d.country_repo.clone(),
        currency_repo: d.currency_repo.clone(),
    }
});

pub struct UpdateCountryHandler {
    country_repo: Arc<dyn CountryRepository>,
    currency_repo: Arc<dyn CurrencyRepository>,
}
#[async_trait]
impl CommandHandler<UpdateCountry> for UpdateCountryHandler {
    async fn handle(&self, cmd: UpdateCountry) -> AppResult<()> {
        let id = CountryCode::new(&cmd.code);
        let mut c = self.country_repo.find_by_id(&id).await?
            .ok_or_else(|| AppError::not_found("Country", id.to_string()))?;
        let cur = CurrencyCode::new(&cmd.currency_code);
        if !cur.0.is_empty() && !self.currency_repo.exists(&cur).await? {
            return Err(AppError::not_found("Currency", cur.to_string()));
        }
        c.update(cmd.name, cur)?;
        self.country_repo.save(&c).await
    }
}
register_command_handler!(UpdateCountry, AppDeps, |d: &AppDeps| {
    UpdateCountryHandler {
        country_repo: d.country_repo.clone(),
        currency_repo: d.currency_repo.clone(),
    }
});

pub struct DeleteCountryHandler { repo: Arc<dyn CountryRepository> }
#[async_trait]
impl CommandHandler<DeleteCountry> for DeleteCountryHandler {
    async fn handle(&self, cmd: DeleteCountry) -> AppResult<()> {
        let id = CountryCode::new(&cmd.code);
        if !self.repo.exists(&id).await? {
            return Err(AppError::not_found("Country", id.to_string()));
        }
        self.repo.delete(&id).await
    }
}
register_command_handler!(DeleteCountry, AppDeps, |d: &AppDeps| {
    DeleteCountryHandler { repo: d.country_repo.clone() }
});

pub struct ActivateCountryHandler { repo: Arc<dyn CountryRepository> }
#[async_trait]
impl CommandHandler<ActivateCountry> for ActivateCountryHandler {
    async fn handle(&self, cmd: ActivateCountry) -> AppResult<()> {
        let id = CountryCode::new(&cmd.code);
        let mut c = self.repo.find_by_id(&id).await?
            .ok_or_else(|| AppError::not_found("Country", id.to_string()))?;
        c.activate();
        self.repo.save(&c).await
    }
}
register_command_handler!(ActivateCountry, AppDeps, |d: &AppDeps| {
    ActivateCountryHandler { repo: d.country_repo.clone() }
});

pub struct DeactivateCountryHandler { repo: Arc<dyn CountryRepository> }
#[async_trait]
impl CommandHandler<DeactivateCountry> for DeactivateCountryHandler {
    async fn handle(&self, cmd: DeactivateCountry) -> AppResult<()> {
        let id = CountryCode::new(&cmd.code);
        let mut c = self.repo.find_by_id(&id).await?
            .ok_or_else(|| AppError::not_found("Country", id.to_string()))?;
        c.deactivate();
        self.repo.save(&c).await
    }
}
register_command_handler!(DeactivateCountry, AppDeps, |d: &AppDeps| {
    DeactivateCountryHandler { repo: d.country_repo.clone() }
});

pub struct GetCountryByCodeHandler { repo: Arc<dyn CountryRepository> }
#[async_trait]
impl QueryHandler<GetCountryByCode> for GetCountryByCodeHandler {
    async fn handle(&self, q: GetCountryByCode) -> AppResult<Option<Country>> {
        self.repo.find_by_id(&CountryCode::new(&q.code)).await
    }
}
register_query_handler!(GetCountryByCode, AppDeps, |d: &AppDeps| {
    GetCountryByCodeHandler { repo: d.country_repo.clone() }
});

pub struct GetAllCountriesHandler { repo: Arc<dyn CountryRepository> }
#[async_trait]
impl QueryHandler<GetAllCountries> for GetAllCountriesHandler {
    async fn handle(&self, _q: GetAllCountries) -> AppResult<Vec<Country>> {
        self.repo.find_all().await
    }
}
register_query_handler!(GetAllCountries, AppDeps, |d: &AppDeps| {
    GetAllCountriesHandler { repo: d.country_repo.clone() }
});

pub struct GetCountriesByCurrencyHandler { repo: Arc<dyn CountryRepository> }
#[async_trait]
impl QueryHandler<GetCountriesByCurrency> for GetCountriesByCurrencyHandler {
    async fn handle(&self, q: GetCountriesByCurrency) -> AppResult<Vec<Country>> {
        self.repo.find_by_currency(&CurrencyCode::new(&q.currency_code)).await
    }
}
register_query_handler!(GetCountriesByCurrency, AppDeps, |d: &AppDeps| {
    GetCountriesByCurrencyHandler { repo: d.country_repo.clone() }
});

// ─── State ───────────────────────────────────────────────────────────────────

pub struct CreateStateHandler {
    state_repo: Arc<dyn StateRepository>,
    country_repo: Arc<dyn CountryRepository>,
}
#[async_trait]
impl CommandHandler<CreateState> for CreateStateHandler {
    async fn handle(&self, cmd: CreateState) -> AppResult<()> {
        let id = StateCode::new(&cmd.code);
        if self.state_repo.exists(&id).await? {
            return Err(AppError::conflict(format!("State '{}' already exists.", id)));
        }
        let country = CountryCode::new(&cmd.country_code);
        if !self.country_repo.exists(&country).await? {
            return Err(AppError::not_found("Country", country.to_string()));
        }
        let s = State::create(id, cmd.name, country)?;
        self.state_repo.save(&s).await
    }
}
register_command_handler!(CreateState, AppDeps, |d: &AppDeps| {
    CreateStateHandler {
        state_repo: d.state_repo.clone(),
        country_repo: d.country_repo.clone(),
    }
});

pub struct UpdateStateHandler { repo: Arc<dyn StateRepository> }
#[async_trait]
impl CommandHandler<UpdateState> for UpdateStateHandler {
    async fn handle(&self, cmd: UpdateState) -> AppResult<()> {
        let id = StateCode::new(&cmd.code);
        let mut s = self.repo.find_by_id(&id).await?
            .ok_or_else(|| AppError::not_found("State", id.to_string()))?;
        s.update(cmd.name)?;
        self.repo.save(&s).await
    }
}
register_command_handler!(UpdateState, AppDeps, |d: &AppDeps| {
    UpdateStateHandler { repo: d.state_repo.clone() }
});

pub struct DeleteStateHandler { repo: Arc<dyn StateRepository> }
#[async_trait]
impl CommandHandler<DeleteState> for DeleteStateHandler {
    async fn handle(&self, cmd: DeleteState) -> AppResult<()> {
        let id = StateCode::new(&cmd.code);
        if !self.repo.exists(&id).await? {
            return Err(AppError::not_found("State", id.to_string()));
        }
        self.repo.delete(&id).await
    }
}
register_command_handler!(DeleteState, AppDeps, |d: &AppDeps| {
    DeleteStateHandler { repo: d.state_repo.clone() }
});

pub struct ActivateStateHandler { repo: Arc<dyn StateRepository> }
#[async_trait]
impl CommandHandler<ActivateState> for ActivateStateHandler {
    async fn handle(&self, cmd: ActivateState) -> AppResult<()> {
        let id = StateCode::new(&cmd.code);
        let mut s = self.repo.find_by_id(&id).await?
            .ok_or_else(|| AppError::not_found("State", id.to_string()))?;
        s.activate();
        self.repo.save(&s).await
    }
}
register_command_handler!(ActivateState, AppDeps, |d: &AppDeps| {
    ActivateStateHandler { repo: d.state_repo.clone() }
});

pub struct DeactivateStateHandler { repo: Arc<dyn StateRepository> }
#[async_trait]
impl CommandHandler<DeactivateState> for DeactivateStateHandler {
    async fn handle(&self, cmd: DeactivateState) -> AppResult<()> {
        let id = StateCode::new(&cmd.code);
        let mut s = self.repo.find_by_id(&id).await?
            .ok_or_else(|| AppError::not_found("State", id.to_string()))?;
        s.deactivate();
        self.repo.save(&s).await
    }
}
register_command_handler!(DeactivateState, AppDeps, |d: &AppDeps| {
    DeactivateStateHandler { repo: d.state_repo.clone() }
});

pub struct GetStateByCodeHandler { repo: Arc<dyn StateRepository> }
#[async_trait]
impl QueryHandler<GetStateByCode> for GetStateByCodeHandler {
    async fn handle(&self, q: GetStateByCode) -> AppResult<Option<State>> {
        self.repo.find_by_id(&StateCode::new(&q.code)).await
    }
}
register_query_handler!(GetStateByCode, AppDeps, |d: &AppDeps| {
    GetStateByCodeHandler { repo: d.state_repo.clone() }
});

pub struct GetAllStatesHandler { repo: Arc<dyn StateRepository> }
#[async_trait]
impl QueryHandler<GetAllStates> for GetAllStatesHandler {
    async fn handle(&self, _q: GetAllStates) -> AppResult<Vec<State>> {
        self.repo.find_all().await
    }
}
register_query_handler!(GetAllStates, AppDeps, |d: &AppDeps| {
    GetAllStatesHandler { repo: d.state_repo.clone() }
});

pub struct GetStatesByCountryHandler { repo: Arc<dyn StateRepository> }
#[async_trait]
impl QueryHandler<GetStatesByCountry> for GetStatesByCountryHandler {
    async fn handle(&self, q: GetStatesByCountry) -> AppResult<Vec<State>> {
        self.repo.find_by_country(&CountryCode::new(&q.country_code)).await
    }
}
register_query_handler!(GetStatesByCountry, AppDeps, |d: &AppDeps| {
    GetStatesByCountryHandler { repo: d.state_repo.clone() }
});

// ─── City ────────────────────────────────────────────────────────────────────

pub struct CreateCityHandler {
    city_repo: Arc<dyn CityRepository>,
    state_repo: Arc<dyn StateRepository>,
}
#[async_trait]
impl CommandHandler<CreateCity> for CreateCityHandler {
    async fn handle(&self, cmd: CreateCity) -> AppResult<()> {
        let id = CityCode::new(&cmd.code);
        if self.city_repo.exists(&id).await? {
            return Err(AppError::conflict(format!("City '{}' already exists.", id)));
        }
        let state = StateCode::new(&cmd.state_code);
        if !self.state_repo.exists(&state).await? {
            return Err(AppError::not_found("State", state.to_string()));
        }
        let c = City::create(id, cmd.name, state)?;
        self.city_repo.save(&c).await
    }
}
register_command_handler!(CreateCity, AppDeps, |d: &AppDeps| {
    CreateCityHandler {
        city_repo: d.city_repo.clone(),
        state_repo: d.state_repo.clone(),
    }
});

pub struct UpdateCityHandler { repo: Arc<dyn CityRepository> }
#[async_trait]
impl CommandHandler<UpdateCity> for UpdateCityHandler {
    async fn handle(&self, cmd: UpdateCity) -> AppResult<()> {
        let id = CityCode::new(&cmd.code);
        let mut c = self.repo.find_by_id(&id).await?
            .ok_or_else(|| AppError::not_found("City", id.to_string()))?;
        c.update(cmd.name)?;
        self.repo.save(&c).await
    }
}
register_command_handler!(UpdateCity, AppDeps, |d: &AppDeps| {
    UpdateCityHandler { repo: d.city_repo.clone() }
});

pub struct DeleteCityHandler { repo: Arc<dyn CityRepository> }
#[async_trait]
impl CommandHandler<DeleteCity> for DeleteCityHandler {
    async fn handle(&self, cmd: DeleteCity) -> AppResult<()> {
        let id = CityCode::new(&cmd.code);
        if !self.repo.exists(&id).await? {
            return Err(AppError::not_found("City", id.to_string()));
        }
        self.repo.delete(&id).await
    }
}
register_command_handler!(DeleteCity, AppDeps, |d: &AppDeps| {
    DeleteCityHandler { repo: d.city_repo.clone() }
});

pub struct ActivateCityHandler { repo: Arc<dyn CityRepository> }
#[async_trait]
impl CommandHandler<ActivateCity> for ActivateCityHandler {
    async fn handle(&self, cmd: ActivateCity) -> AppResult<()> {
        let id = CityCode::new(&cmd.code);
        let mut c = self.repo.find_by_id(&id).await?
            .ok_or_else(|| AppError::not_found("City", id.to_string()))?;
        c.activate();
        self.repo.save(&c).await
    }
}
register_command_handler!(ActivateCity, AppDeps, |d: &AppDeps| {
    ActivateCityHandler { repo: d.city_repo.clone() }
});

pub struct DeactivateCityHandler { repo: Arc<dyn CityRepository> }
#[async_trait]
impl CommandHandler<DeactivateCity> for DeactivateCityHandler {
    async fn handle(&self, cmd: DeactivateCity) -> AppResult<()> {
        let id = CityCode::new(&cmd.code);
        let mut c = self.repo.find_by_id(&id).await?
            .ok_or_else(|| AppError::not_found("City", id.to_string()))?;
        c.deactivate();
        self.repo.save(&c).await
    }
}
register_command_handler!(DeactivateCity, AppDeps, |d: &AppDeps| {
    DeactivateCityHandler { repo: d.city_repo.clone() }
});

pub struct GetCityByCodeHandler { repo: Arc<dyn CityRepository> }
#[async_trait]
impl QueryHandler<GetCityByCode> for GetCityByCodeHandler {
    async fn handle(&self, q: GetCityByCode) -> AppResult<Option<City>> {
        self.repo.find_by_id(&CityCode::new(&q.code)).await
    }
}
register_query_handler!(GetCityByCode, AppDeps, |d: &AppDeps| {
    GetCityByCodeHandler { repo: d.city_repo.clone() }
});

pub struct GetAllCitiesHandler { repo: Arc<dyn CityRepository> }
#[async_trait]
impl QueryHandler<GetAllCities> for GetAllCitiesHandler {
    async fn handle(&self, _q: GetAllCities) -> AppResult<Vec<City>> {
        self.repo.find_all().await
    }
}
register_query_handler!(GetAllCities, AppDeps, |d: &AppDeps| {
    GetAllCitiesHandler { repo: d.city_repo.clone() }
});

pub struct GetCitiesByStateHandler { repo: Arc<dyn CityRepository> }
#[async_trait]
impl QueryHandler<GetCitiesByState> for GetCitiesByStateHandler {
    async fn handle(&self, q: GetCitiesByState) -> AppResult<Vec<City>> {
        self.repo.find_by_state(&StateCode::new(&q.state_code)).await
    }
}
register_query_handler!(GetCitiesByState, AppDeps, |d: &AppDeps| {
    GetCitiesByStateHandler { repo: d.city_repo.clone() }
});

// ─── Pincode ─────────────────────────────────────────────────────────────────

pub struct CreatePincodeHandler {
    pincode_repo: Arc<dyn PincodeRepository>,
    city_repo: Arc<dyn CityRepository>,
}
#[async_trait]
impl CommandHandler<CreatePincode> for CreatePincodeHandler {
    async fn handle(&self, cmd: CreatePincode) -> AppResult<()> {
        let id = PincodeId::new(&cmd.code);
        if self.pincode_repo.exists(&id).await? {
            return Err(AppError::conflict(format!("Pincode '{}' already exists.", id)));
        }
        let city = CityCode::new(&cmd.city_code);
        if !self.city_repo.exists(&city).await? {
            return Err(AppError::not_found("City", city.to_string()));
        }
        let p = Pincode::create(id, city, cmd.area_name)?;
        self.pincode_repo.save(&p).await
    }
}
register_command_handler!(CreatePincode, AppDeps, |d: &AppDeps| {
    CreatePincodeHandler {
        pincode_repo: d.pincode_repo.clone(),
        city_repo: d.city_repo.clone(),
    }
});

pub struct UpdatePincodeHandler { repo: Arc<dyn PincodeRepository> }
#[async_trait]
impl CommandHandler<UpdatePincode> for UpdatePincodeHandler {
    async fn handle(&self, cmd: UpdatePincode) -> AppResult<()> {
        let id = PincodeId::new(&cmd.code);
        let mut p = self.repo.find_by_id(&id).await?
            .ok_or_else(|| AppError::not_found("Pincode", id.to_string()))?;
        p.update(cmd.area_name);
        self.repo.save(&p).await
    }
}
register_command_handler!(UpdatePincode, AppDeps, |d: &AppDeps| {
    UpdatePincodeHandler { repo: d.pincode_repo.clone() }
});

pub struct DeletePincodeHandler { repo: Arc<dyn PincodeRepository> }
#[async_trait]
impl CommandHandler<DeletePincode> for DeletePincodeHandler {
    async fn handle(&self, cmd: DeletePincode) -> AppResult<()> {
        let id = PincodeId::new(&cmd.code);
        if !self.repo.exists(&id).await? {
            return Err(AppError::not_found("Pincode", id.to_string()));
        }
        self.repo.delete(&id).await
    }
}
register_command_handler!(DeletePincode, AppDeps, |d: &AppDeps| {
    DeletePincodeHandler { repo: d.pincode_repo.clone() }
});

pub struct ActivatePincodeHandler { repo: Arc<dyn PincodeRepository> }
#[async_trait]
impl CommandHandler<ActivatePincode> for ActivatePincodeHandler {
    async fn handle(&self, cmd: ActivatePincode) -> AppResult<()> {
        let id = PincodeId::new(&cmd.code);
        let mut p = self.repo.find_by_id(&id).await?
            .ok_or_else(|| AppError::not_found("Pincode", id.to_string()))?;
        p.activate();
        self.repo.save(&p).await
    }
}
register_command_handler!(ActivatePincode, AppDeps, |d: &AppDeps| {
    ActivatePincodeHandler { repo: d.pincode_repo.clone() }
});

pub struct DeactivatePincodeHandler { repo: Arc<dyn PincodeRepository> }
#[async_trait]
impl CommandHandler<DeactivatePincode> for DeactivatePincodeHandler {
    async fn handle(&self, cmd: DeactivatePincode) -> AppResult<()> {
        let id = PincodeId::new(&cmd.code);
        let mut p = self.repo.find_by_id(&id).await?
            .ok_or_else(|| AppError::not_found("Pincode", id.to_string()))?;
        p.deactivate();
        self.repo.save(&p).await
    }
}
register_command_handler!(DeactivatePincode, AppDeps, |d: &AppDeps| {
    DeactivatePincodeHandler { repo: d.pincode_repo.clone() }
});

pub struct GetPincodeByCodeHandler { repo: Arc<dyn PincodeRepository> }
#[async_trait]
impl QueryHandler<GetPincodeByCode> for GetPincodeByCodeHandler {
    async fn handle(&self, q: GetPincodeByCode) -> AppResult<Option<Pincode>> {
        self.repo.find_by_id(&PincodeId::new(&q.code)).await
    }
}
register_query_handler!(GetPincodeByCode, AppDeps, |d: &AppDeps| {
    GetPincodeByCodeHandler { repo: d.pincode_repo.clone() }
});

pub struct GetAllPincodesHandler { repo: Arc<dyn PincodeRepository> }
#[async_trait]
impl QueryHandler<GetAllPincodes> for GetAllPincodesHandler {
    async fn handle(&self, _q: GetAllPincodes) -> AppResult<Vec<Pincode>> {
        self.repo.find_all().await
    }
}
register_query_handler!(GetAllPincodes, AppDeps, |d: &AppDeps| {
    GetAllPincodesHandler { repo: d.pincode_repo.clone() }
});

pub struct GetPincodesByCityHandler { repo: Arc<dyn PincodeRepository> }
#[async_trait]
impl QueryHandler<GetPincodesByCity> for GetPincodesByCityHandler {
    async fn handle(&self, q: GetPincodesByCity) -> AppResult<Vec<Pincode>> {
        self.repo.find_by_city(&CityCode::new(&q.city_code)).await
    }
}
register_query_handler!(GetPincodesByCity, AppDeps, |d: &AppDeps| {
    GetPincodesByCityHandler { repo: d.pincode_repo.clone() }
});
