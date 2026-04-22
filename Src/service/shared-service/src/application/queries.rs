use crate::domain::entities::{City, Country, Currency, Pincode, State};

// ── Currency queries ─────────────────────────────────────────────────────────

pub struct GetCurrencyByCode { pub code: String }
ddd_application::impl_query!(GetCurrencyByCode, Option<Currency>);

pub struct GetAllCurrencies;
ddd_application::impl_query!(GetAllCurrencies, Vec<Currency>);

// ── Country queries ──────────────────────────────────────────────────────────

pub struct GetCountryByCode { pub code: String }
ddd_application::impl_query!(GetCountryByCode, Option<Country>);

pub struct GetAllCountries;
ddd_application::impl_query!(GetAllCountries, Vec<Country>);

pub struct GetCountriesByCurrency { pub currency_code: String }
ddd_application::impl_query!(GetCountriesByCurrency, Vec<Country>);

// ── State queries ────────────────────────────────────────────────────────────

pub struct GetStateByCode { pub code: String }
ddd_application::impl_query!(GetStateByCode, Option<State>);

pub struct GetAllStates;
ddd_application::impl_query!(GetAllStates, Vec<State>);

pub struct GetStatesByCountry { pub country_code: String }
ddd_application::impl_query!(GetStatesByCountry, Vec<State>);

// ── City queries ─────────────────────────────────────────────────────────────

pub struct GetCityByCode { pub code: String }
ddd_application::impl_query!(GetCityByCode, Option<City>);

pub struct GetAllCities;
ddd_application::impl_query!(GetAllCities, Vec<City>);

pub struct GetCitiesByState { pub state_code: String }
ddd_application::impl_query!(GetCitiesByState, Vec<City>);

// ── Pincode queries ──────────────────────────────────────────────────────────

pub struct GetPincodeByCode { pub code: String }
ddd_application::impl_query!(GetPincodeByCode, Option<Pincode>);

pub struct GetAllPincodes;
ddd_application::impl_query!(GetAllPincodes, Vec<Pincode>);

pub struct GetPincodesByCity { pub city_code: String }
ddd_application::impl_query!(GetPincodesByCity, Vec<Pincode>);
