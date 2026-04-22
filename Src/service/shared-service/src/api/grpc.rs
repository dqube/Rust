//! gRPC server implementation for SharedService.

use std::sync::Arc;

use ddd_api::grpc::error::GrpcErrorExt;
use ddd_application::Mediator;
use tonic::{Request, Response, Status};

use crate::application::commands::*;
use crate::application::queries::*;
use crate::domain::entities::{City, Country, Currency, Pincode, State};
use crate::proto::{
    shared_service_server::{SharedService, SharedServiceServer},
    CityInfo, CityListResponse, CityResponse, CountryInfo, CountryListResponse, CountryResponse,
    CreateCityRequest, CreateCountryRequest, CreateCurrencyRequest, CreatePincodeRequest,
    CreateStateRequest, CurrencyInfo, CurrencyListResponse, CurrencyResponse, Empty,
    GetByCodeRequest, PincodeInfo, PincodeListResponse, PincodeResponse, StateInfo,
    StateListResponse, StateResponse, UpdateCityRequest, UpdateCountryRequest,
    UpdateCurrencyRequest, UpdatePincodeRequest, UpdateStateRequest,
};

#[derive(Clone)]
pub struct SharedGrpcService {
    mediator: Arc<Mediator>,
}

impl SharedGrpcService {
    pub fn new(mediator: Arc<Mediator>) -> Self {
        Self { mediator }
    }
    pub fn into_server(self) -> SharedServiceServer<Self> {
        SharedServiceServer::new(self)
    }
}

// ── Mappers ──────────────────────────────────────────────────────────────────

fn fmt(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.to_rfc3339()
}
fn fmt_opt(dt: Option<chrono::DateTime<chrono::Utc>>) -> String {
    dt.map(fmt).unwrap_or_default()
}

fn to_currency(c: &Currency) -> CurrencyInfo {
    CurrencyInfo {
        code: c.id.0.clone(),
        name: c.name.clone(),
        symbol: c.symbol.clone(),
        created_at: fmt(c.created_at),
        updated_at: fmt_opt(c.updated_at),
        is_active: c.is_active,
    }
}
fn to_country(c: &Country) -> CountryInfo {
    CountryInfo {
        code: c.id.0.clone(),
        name: c.name.clone(),
        currency_code: c.currency_code.0.clone(),
        created_at: fmt(c.created_at),
        updated_at: fmt_opt(c.updated_at),
        is_active: c.is_active,
    }
}
fn to_state(s: &State) -> StateInfo {
    StateInfo {
        code: s.id.0.clone(),
        name: s.name.clone(),
        country_code: s.country_code.0.clone(),
        created_at: fmt(s.created_at),
        updated_at: fmt_opt(s.updated_at),
        is_active: s.is_active,
    }
}
fn to_city(c: &City) -> CityInfo {
    CityInfo {
        code: c.id.0.clone(),
        name: c.name.clone(),
        state_code: c.state_code.0.clone(),
        created_at: fmt(c.created_at),
        updated_at: fmt_opt(c.updated_at),
        is_active: c.is_active,
    }
}
fn to_pincode(p: &Pincode) -> PincodeInfo {
    PincodeInfo {
        code: p.id.0.clone(),
        city_code: p.city_code.0.clone(),
        area_name: p.area_name.clone().unwrap_or_default(),
        created_at: fmt(p.created_at),
        updated_at: fmt_opt(p.updated_at),
        is_active: p.is_active,
    }
}

#[tonic::async_trait]
impl SharedService for SharedGrpcService {
    // ── Currencies ──────────────────────────────────────────────────────────

    async fn get_all_currencies(
        &self,
        _req: Request<Empty>,
    ) -> Result<Response<CurrencyListResponse>, Status> {
        let items = self
            .mediator
            .query(GetAllCurrencies)
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CurrencyListResponse {
            currencies: items.iter().map(to_currency).collect(),
        }))
    }
    async fn get_currency_by_code(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<CurrencyResponse>, Status> {
        let r = req.into_inner();
        let found = self
            .mediator
            .query(GetCurrencyByCode { code: r.code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CurrencyResponse {
            found: found.is_some(),
            currency: found.as_ref().map(to_currency),
        }))
    }
    async fn create_currency(
        &self,
        req: Request<CreateCurrencyRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        self.mediator
            .send(CreateCurrency { code: r.code, name: r.name, symbol: r.symbol })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn update_currency(
        &self,
        req: Request<UpdateCurrencyRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        self.mediator
            .send(UpdateCurrency { code: r.code, name: r.name, symbol: r.symbol })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn delete_currency(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<Empty>, Status> {
        self.mediator
            .send(DeleteCurrency { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn activate_currency(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<Empty>, Status> {
        self.mediator
            .send(ActivateCurrency { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn deactivate_currency(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<Empty>, Status> {
        self.mediator
            .send(DeactivateCurrency { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }

    // ── Countries ───────────────────────────────────────────────────────────

    async fn get_all_countries(
        &self,
        _req: Request<Empty>,
    ) -> Result<Response<CountryListResponse>, Status> {
        let items = self.mediator.query(GetAllCountries).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CountryListResponse {
            countries: items.iter().map(to_country).collect(),
        }))
    }
    async fn get_country_by_code(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<CountryResponse>, Status> {
        let found = self
            .mediator
            .query(GetCountryByCode { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CountryResponse {
            found: found.is_some(),
            country: found.as_ref().map(to_country),
        }))
    }
    async fn get_countries_by_currency(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<CountryListResponse>, Status> {
        let items = self
            .mediator
            .query(GetCountriesByCurrency { currency_code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CountryListResponse {
            countries: items.iter().map(to_country).collect(),
        }))
    }
    async fn create_country(
        &self,
        req: Request<CreateCountryRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        self.mediator
            .send(CreateCountry { code: r.code, name: r.name, currency_code: r.currency_code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn update_country(
        &self,
        req: Request<UpdateCountryRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        self.mediator
            .send(UpdateCountry { code: r.code, name: r.name, currency_code: r.currency_code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn delete_country(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<Empty>, Status> {
        self.mediator
            .send(DeleteCountry { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn activate_country(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<Empty>, Status> {
        self.mediator
            .send(ActivateCountry { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn deactivate_country(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<Empty>, Status> {
        self.mediator
            .send(DeactivateCountry { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }

    // ── States ──────────────────────────────────────────────────────────────

    async fn get_all_states(
        &self,
        _req: Request<Empty>,
    ) -> Result<Response<StateListResponse>, Status> {
        let items = self.mediator.query(GetAllStates).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(StateListResponse {
            states: items.iter().map(to_state).collect(),
        }))
    }
    async fn get_state_by_code(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<StateResponse>, Status> {
        let found = self
            .mediator
            .query(GetStateByCode { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(StateResponse {
            found: found.is_some(),
            state: found.as_ref().map(to_state),
        }))
    }
    async fn get_states_by_country(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<StateListResponse>, Status> {
        let items = self
            .mediator
            .query(GetStatesByCountry { country_code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(StateListResponse {
            states: items.iter().map(to_state).collect(),
        }))
    }
    async fn create_state(
        &self,
        req: Request<CreateStateRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        self.mediator
            .send(CreateState { code: r.code, name: r.name, country_code: r.country_code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn update_state(
        &self,
        req: Request<UpdateStateRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        self.mediator
            .send(UpdateState { code: r.code, name: r.name })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn delete_state(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<Empty>, Status> {
        self.mediator
            .send(DeleteState { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn activate_state(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<Empty>, Status> {
        self.mediator
            .send(ActivateState { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn deactivate_state(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<Empty>, Status> {
        self.mediator
            .send(DeactivateState { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }

    // ── Cities ──────────────────────────────────────────────────────────────

    async fn get_all_cities(
        &self,
        _req: Request<Empty>,
    ) -> Result<Response<CityListResponse>, Status> {
        let items = self.mediator.query(GetAllCities).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CityListResponse {
            cities: items.iter().map(to_city).collect(),
        }))
    }
    async fn get_city_by_code(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<CityResponse>, Status> {
        let found = self
            .mediator
            .query(GetCityByCode { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CityResponse {
            found: found.is_some(),
            city: found.as_ref().map(to_city),
        }))
    }
    async fn get_cities_by_state(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<CityListResponse>, Status> {
        let items = self
            .mediator
            .query(GetCitiesByState { state_code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(CityListResponse {
            cities: items.iter().map(to_city).collect(),
        }))
    }
    async fn create_city(
        &self,
        req: Request<CreateCityRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        self.mediator
            .send(CreateCity { code: r.code, name: r.name, state_code: r.state_code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn update_city(
        &self,
        req: Request<UpdateCityRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        self.mediator
            .send(UpdateCity { code: r.code, name: r.name })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn delete_city(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<Empty>, Status> {
        self.mediator
            .send(DeleteCity { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn activate_city(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<Empty>, Status> {
        self.mediator
            .send(ActivateCity { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn deactivate_city(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<Empty>, Status> {
        self.mediator
            .send(DeactivateCity { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }

    // ── Pincodes ────────────────────────────────────────────────────────────

    async fn get_all_pincodes(
        &self,
        _req: Request<Empty>,
    ) -> Result<Response<PincodeListResponse>, Status> {
        let items = self.mediator.query(GetAllPincodes).await.map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(PincodeListResponse {
            pincodes: items.iter().map(to_pincode).collect(),
        }))
    }
    async fn get_pincode_by_code(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<PincodeResponse>, Status> {
        let found = self
            .mediator
            .query(GetPincodeByCode { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(PincodeResponse {
            found: found.is_some(),
            pincode: found.as_ref().map(to_pincode),
        }))
    }
    async fn get_pincodes_by_city(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<PincodeListResponse>, Status> {
        let items = self
            .mediator
            .query(GetPincodesByCity { city_code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(PincodeListResponse {
            pincodes: items.iter().map(to_pincode).collect(),
        }))
    }
    async fn create_pincode(
        &self,
        req: Request<CreatePincodeRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let area = if r.area_name.is_empty() { None } else { Some(r.area_name) };
        self.mediator
            .send(CreatePincode { code: r.code, city_code: r.city_code, area_name: area })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn update_pincode(
        &self,
        req: Request<UpdatePincodeRequest>,
    ) -> Result<Response<Empty>, Status> {
        let r = req.into_inner();
        let area = if r.area_name.is_empty() { None } else { Some(r.area_name) };
        self.mediator
            .send(UpdatePincode { code: r.code, area_name: area })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn delete_pincode(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<Empty>, Status> {
        self.mediator
            .send(DeletePincode { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn activate_pincode(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<Empty>, Status> {
        self.mediator
            .send(ActivatePincode { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
    async fn deactivate_pincode(
        &self,
        req: Request<GetByCodeRequest>,
    ) -> Result<Response<Empty>, Status> {
        self.mediator
            .send(DeactivatePincode { code: req.into_inner().code })
            .await
            .map_err(|e| e.to_grpc_status())?;
        Ok(Response::new(Empty {}))
    }
}
