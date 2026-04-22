use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ddd_shared_kernel::{AppError, AppResult};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, Set,
};

use super::models::*;
use crate::domain::entities::{City, Country, Currency, Pincode, State};
use crate::domain::ids::{CityCode, CountryCode, CurrencyCode, PincodeId, StateCode};
use crate::domain::repositories::{
    CityRepository, CountryRepository, CurrencyRepository, PincodeRepository, StateRepository,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn to_utc(dt: sea_orm::prelude::DateTimeWithTimeZone) -> DateTime<Utc> {
    dt.with_timezone(&Utc)
}

fn opt_to_utc(dt: Option<sea_orm::prelude::DateTimeWithTimeZone>) -> Option<DateTime<Utc>> {
    dt.map(to_utc)
}

fn from_utc(dt: DateTime<Utc>) -> sea_orm::prelude::DateTimeWithTimeZone {
    dt.fixed_offset()
}

fn opt_from_utc(dt: Option<DateTime<Utc>>) -> Option<sea_orm::prelude::DateTimeWithTimeZone> {
    dt.map(|d| d.fixed_offset())
}

fn db_err(e: sea_orm::DbErr) -> AppError {
    AppError::internal(format!("db: {e}"))
}

// ── Mappers ──────────────────────────────────────────────────────────────────

fn to_currency(m: currency::Model) -> Currency {
    Currency {
        id: CurrencyCode(m.id),
        name: m.name,
        symbol: m.symbol,
        is_active: m.is_active,
        created_at: to_utc(m.created_at),
        updated_at: opt_to_utc(m.updated_at),
    }
}

fn to_country(m: country::Model) -> Country {
    Country {
        id: CountryCode(m.id),
        name: m.name,
        currency_code: CurrencyCode(m.currency_code),
        is_active: m.is_active,
        created_at: to_utc(m.created_at),
        updated_at: opt_to_utc(m.updated_at),
    }
}

fn to_state(m: state::Model) -> State {
    State {
        id: StateCode(m.id),
        name: m.name,
        country_code: CountryCode(m.country_code),
        is_active: m.is_active,
        created_at: to_utc(m.created_at),
        updated_at: opt_to_utc(m.updated_at),
    }
}

fn to_city(m: city::Model) -> City {
    City {
        id: CityCode(m.id),
        name: m.name,
        state_code: StateCode(m.state_code),
        is_active: m.is_active,
        created_at: to_utc(m.created_at),
        updated_at: opt_to_utc(m.updated_at),
    }
}

fn to_pincode(m: pincode::Model) -> Pincode {
    Pincode {
        id: PincodeId(m.id),
        city_code: CityCode(m.city_code),
        area_name: m.area_name,
        is_active: m.is_active,
        created_at: to_utc(m.created_at),
        updated_at: opt_to_utc(m.updated_at),
    }
}

// ── PgCurrencyRepository ─────────────────────────────────────────────────────

pub struct PgCurrencyRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl CurrencyRepository for PgCurrencyRepository {
    async fn find_by_id(&self, id: &CurrencyCode) -> AppResult<Option<Currency>> {
        Ok(currency::Entity::find_by_id(&id.0)
            .one(&*self.0).await.map_err(db_err)?
            .map(to_currency))
    }
    async fn find_all(&self) -> AppResult<Vec<Currency>> {
        Ok(currency::Entity::find()
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(to_currency).collect())
    }
    async fn exists(&self, id: &CurrencyCode) -> AppResult<bool> {
        Ok(currency::Entity::find_by_id(&id.0)
            .count(&*self.0).await.map_err(db_err)? > 0)
    }
    async fn save(&self, c: &Currency) -> AppResult<()> {
        let active = currency::ActiveModel {
            id: Set(c.id.0.clone()),
            name: Set(c.name.clone()),
            symbol: Set(c.symbol.clone()),
            is_active: Set(c.is_active),
            created_at: Set(from_utc(c.created_at)),
            updated_at: Set(opt_from_utc(c.updated_at)),
        };
        currency::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(currency::Column::Id)
                    .update_columns([
                        currency::Column::Name,
                        currency::Column::Symbol,
                        currency::Column::IsActive,
                        currency::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
    async fn delete(&self, id: &CurrencyCode) -> AppResult<()> {
        currency::Entity::delete_by_id(&id.0).exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
}

// ── PgCountryRepository ──────────────────────────────────────────────────────

pub struct PgCountryRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl CountryRepository for PgCountryRepository {
    async fn find_by_id(&self, id: &CountryCode) -> AppResult<Option<Country>> {
        Ok(country::Entity::find_by_id(&id.0)
            .one(&*self.0).await.map_err(db_err)?
            .map(to_country))
    }
    async fn find_all(&self) -> AppResult<Vec<Country>> {
        Ok(country::Entity::find()
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(to_country).collect())
    }
    async fn find_by_currency(&self, currency_code: &CurrencyCode) -> AppResult<Vec<Country>> {
        Ok(country::Entity::find()
            .filter(country::Column::CurrencyCode.eq(&currency_code.0))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(to_country).collect())
    }
    async fn exists(&self, id: &CountryCode) -> AppResult<bool> {
        Ok(country::Entity::find_by_id(&id.0)
            .count(&*self.0).await.map_err(db_err)? > 0)
    }
    async fn save(&self, c: &Country) -> AppResult<()> {
        let active = country::ActiveModel {
            id: Set(c.id.0.clone()),
            name: Set(c.name.clone()),
            currency_code: Set(c.currency_code.0.clone()),
            is_active: Set(c.is_active),
            created_at: Set(from_utc(c.created_at)),
            updated_at: Set(opt_from_utc(c.updated_at)),
        };
        country::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(country::Column::Id)
                    .update_columns([
                        country::Column::Name,
                        country::Column::CurrencyCode,
                        country::Column::IsActive,
                        country::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
    async fn delete(&self, id: &CountryCode) -> AppResult<()> {
        country::Entity::delete_by_id(&id.0).exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
}

// ── PgStateRepository ────────────────────────────────────────────────────────

pub struct PgStateRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl StateRepository for PgStateRepository {
    async fn find_by_id(&self, id: &StateCode) -> AppResult<Option<State>> {
        Ok(state::Entity::find_by_id(&id.0)
            .one(&*self.0).await.map_err(db_err)?
            .map(to_state))
    }
    async fn find_all(&self) -> AppResult<Vec<State>> {
        Ok(state::Entity::find()
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(to_state).collect())
    }
    async fn find_by_country(&self, country_code: &CountryCode) -> AppResult<Vec<State>> {
        Ok(state::Entity::find()
            .filter(state::Column::CountryCode.eq(&country_code.0))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(to_state).collect())
    }
    async fn exists(&self, id: &StateCode) -> AppResult<bool> {
        Ok(state::Entity::find_by_id(&id.0)
            .count(&*self.0).await.map_err(db_err)? > 0)
    }
    async fn save(&self, s: &State) -> AppResult<()> {
        let active = state::ActiveModel {
            id: Set(s.id.0.clone()),
            name: Set(s.name.clone()),
            country_code: Set(s.country_code.0.clone()),
            is_active: Set(s.is_active),
            created_at: Set(from_utc(s.created_at)),
            updated_at: Set(opt_from_utc(s.updated_at)),
        };
        state::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(state::Column::Id)
                    .update_columns([
                        state::Column::Name,
                        state::Column::IsActive,
                        state::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
    async fn delete(&self, id: &StateCode) -> AppResult<()> {
        state::Entity::delete_by_id(&id.0).exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
}

// ── PgCityRepository ─────────────────────────────────────────────────────────

pub struct PgCityRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl CityRepository for PgCityRepository {
    async fn find_by_id(&self, id: &CityCode) -> AppResult<Option<City>> {
        Ok(city::Entity::find_by_id(&id.0)
            .one(&*self.0).await.map_err(db_err)?
            .map(to_city))
    }
    async fn find_all(&self) -> AppResult<Vec<City>> {
        Ok(city::Entity::find()
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(to_city).collect())
    }
    async fn find_by_state(&self, state_code: &StateCode) -> AppResult<Vec<City>> {
        Ok(city::Entity::find()
            .filter(city::Column::StateCode.eq(&state_code.0))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(to_city).collect())
    }
    async fn exists(&self, id: &CityCode) -> AppResult<bool> {
        Ok(city::Entity::find_by_id(&id.0)
            .count(&*self.0).await.map_err(db_err)? > 0)
    }
    async fn save(&self, c: &City) -> AppResult<()> {
        let active = city::ActiveModel {
            id: Set(c.id.0.clone()),
            name: Set(c.name.clone()),
            state_code: Set(c.state_code.0.clone()),
            is_active: Set(c.is_active),
            created_at: Set(from_utc(c.created_at)),
            updated_at: Set(opt_from_utc(c.updated_at)),
        };
        city::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(city::Column::Id)
                    .update_columns([
                        city::Column::Name,
                        city::Column::IsActive,
                        city::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
    async fn delete(&self, id: &CityCode) -> AppResult<()> {
        city::Entity::delete_by_id(&id.0).exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
}

// ── PgPincodeRepository ──────────────────────────────────────────────────────

pub struct PgPincodeRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl PincodeRepository for PgPincodeRepository {
    async fn find_by_id(&self, id: &PincodeId) -> AppResult<Option<Pincode>> {
        Ok(pincode::Entity::find_by_id(&id.0)
            .one(&*self.0).await.map_err(db_err)?
            .map(to_pincode))
    }
    async fn find_all(&self) -> AppResult<Vec<Pincode>> {
        Ok(pincode::Entity::find()
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(to_pincode).collect())
    }
    async fn find_by_city(&self, city_code: &CityCode) -> AppResult<Vec<Pincode>> {
        Ok(pincode::Entity::find()
            .filter(pincode::Column::CityCode.eq(&city_code.0))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(to_pincode).collect())
    }
    async fn exists(&self, id: &PincodeId) -> AppResult<bool> {
        Ok(pincode::Entity::find_by_id(&id.0)
            .count(&*self.0).await.map_err(db_err)? > 0)
    }
    async fn save(&self, p: &Pincode) -> AppResult<()> {
        let active = pincode::ActiveModel {
            id: Set(p.id.0.clone()),
            city_code: Set(p.city_code.0.clone()),
            area_name: Set(p.area_name.clone()),
            is_active: Set(p.is_active),
            created_at: Set(from_utc(p.created_at)),
            updated_at: Set(opt_from_utc(p.updated_at)),
        };
        pincode::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(pincode::Column::Id)
                    .update_columns([
                        pincode::Column::AreaName,
                        pincode::Column::IsActive,
                        pincode::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
    async fn delete(&self, id: &PincodeId) -> AppResult<()> {
        pincode::Entity::delete_by_id(&id.0).exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
}
