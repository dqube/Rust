use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveValue, ColumnTrait, Condition, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QuerySelect, Set,
};
use std::sync::Arc;

use ddd_shared_kernel::AppError;

use crate::domain::{
    entities::{Register, Store, StoreSchedule},
    enums::{RegisterStatus, StoreStatus},
    ids::{RegisterId, StoreId},
    repositories::{PagedResult, RegisterRepository, StoreRepository},
};
use super::models::*;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn db_err(e: sea_orm::DbErr) -> AppError {
    AppError::internal(e.to_string())
}

fn to_utc(dt: sea_orm::prelude::DateTimeWithTimeZone) -> DateTime<Utc> {
    dt.with_timezone(&Utc)
}

fn opt_to_utc(dt: Option<sea_orm::prelude::DateTimeWithTimeZone>) -> Option<DateTime<Utc>> {
    dt.map(to_utc)
}

fn from_utc(dt: DateTime<Utc>) -> sea_orm::prelude::DateTimeWithTimeZone {
    dt.fixed_offset()
}

// ── Model → domain mappers ────────────────────────────────────────────────────

fn model_to_store(m: store::Model) -> Store {
    let schedules: Vec<StoreSchedule> = serde_json::from_value(m.schedules).unwrap_or_default();
    let services:  Vec<String>        = serde_json::from_value(m.services).unwrap_or_default();
    let created_at = to_utc(m.created_at);
    Store {
        id:                  StoreId(m.id),
        version:             0,
        domain_events:       Vec::new(),
        name:                m.name,
        location_id:         m.location_id,
        address_street:      m.address_street,
        address_city:        m.address_city,
        address_postal_code: m.address_postal_code,
        address_country:     m.address_country,
        phone:               m.phone,
        geo_latitude:        m.geo_latitude,
        geo_longitude:       m.geo_longitude,
        schedules,
        services,
        status:              StoreStatus::from_str(&m.status),
        logo_object_name:    m.logo_object_name,
        created_at,
        updated_at:          opt_to_utc(m.updated_at).unwrap_or(created_at),
    }
}

fn model_to_register(m: register::Model) -> Register {
    let created_at = to_utc(m.created_at);
    Register {
        id:              RegisterId(m.id),
        version:         0,
        domain_events:   Vec::new(),
        store_id:        StoreId(m.store_id),
        name:            m.name,
        current_balance: m.current_balance,
        status:          RegisterStatus::from_str(&m.status),
        last_open:       opt_to_utc(m.last_open),
        last_close:      opt_to_utc(m.last_close),
        created_at,
        updated_at:      opt_to_utc(m.updated_at).unwrap_or(created_at),
    }
}

// ── PgStoreRepository ─────────────────────────────────────────────────────────

pub struct PgStoreRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl StoreRepository for PgStoreRepository {
    async fn find_by_id(&self, id: StoreId) -> Result<Option<Store>, AppError> {
        Ok(store::Entity::find_by_id(id.0)
            .one(&*self.0).await.map_err(db_err)?
            .map(model_to_store))
    }

    async fn name_exists(&self, name: &str) -> Result<bool, AppError> {
        let count = store::Entity::find()
            .filter(store::Column::Name.eq(name.to_string()))
            .count(&*self.0).await.map_err(db_err)?;
        Ok(count > 0)
    }

    async fn get_paged(
        &self,
        page:        i32,
        page_size:   i32,
        search:      Option<&str>,
        status:      Option<StoreStatus>,
        location_id: Option<i32>,
    ) -> Result<PagedResult<Store>, AppError> {
        let mut q = store::Entity::find();
        if let Some(s) = search {
            let pat = format!("%{}%", s.to_lowercase());
            q = q.filter(store::Column::Name.contains(pat));
        }
        if let Some(st) = status {
            q = q.filter(store::Column::Status.eq(st.as_str().to_string()));
        }
        if let Some(loc) = location_id {
            q = q.filter(store::Column::LocationId.eq(loc));
        }
        let offset = ((page - 1) * page_size) as u64;
        let limit  = page_size as u64;
        let total  = q.clone().count(&*self.0).await.map_err(db_err)? as i64;
        let items  = q.offset(offset).limit(limit).all(&*self.0).await.map_err(db_err)?
            .into_iter().map(model_to_store).collect();
        Ok(PagedResult::new(items, total, page, page_size))
    }

    async fn save(&self, store: &mut Store) -> Result<(), AppError> {
        let schedules_json = serde_json::to_value(&store.schedules)
            .map_err(|e| AppError::internal(e.to_string()))?;
        let services_json = serde_json::to_value(&store.services)
            .map_err(|e| AppError::internal(e.to_string()))?;

        if store.id.0 == 0 {
            let active = store::ActiveModel {
                id:                  ActiveValue::NotSet,
                name:                Set(store.name.clone()),
                location_id:         Set(store.location_id),
                address_street:      Set(store.address_street.clone()),
                address_city:        Set(store.address_city.clone()),
                address_postal_code: Set(store.address_postal_code.clone()),
                address_country:     Set(store.address_country.clone()),
                phone:               Set(store.phone.clone()),
                geo_latitude:        Set(store.geo_latitude),
                geo_longitude:       Set(store.geo_longitude),
                schedules:           Set(schedules_json),
                services:            Set(services_json),
                status:              Set(store.status.as_str().to_string()),
                logo_object_name:    Set(store.logo_object_name.clone()),
                created_at:          Set(from_utc(store.created_at)),
                updated_at:          Set(Some(from_utc(store.updated_at))),
            };
            let result = store::Entity::insert(active).exec(&*self.0).await.map_err(db_err)?;
            store.id = StoreId(result.last_insert_id);
        } else {
            let active = store::ActiveModel {
                id:                  Set(store.id.0),
                name:                Set(store.name.clone()),
                location_id:         Set(store.location_id),
                address_street:      Set(store.address_street.clone()),
                address_city:        Set(store.address_city.clone()),
                address_postal_code: Set(store.address_postal_code.clone()),
                address_country:     Set(store.address_country.clone()),
                phone:               Set(store.phone.clone()),
                geo_latitude:        Set(store.geo_latitude),
                geo_longitude:       Set(store.geo_longitude),
                schedules:           Set(schedules_json),
                services:            Set(services_json),
                status:              Set(store.status.as_str().to_string()),
                logo_object_name:    Set(store.logo_object_name.clone()),
                created_at:          Set(from_utc(store.created_at)),
                updated_at:          Set(Some(from_utc(store.updated_at))),
            };
            store::Entity::insert(active)
                .on_conflict(
                    sea_orm::sea_query::OnConflict::column(store::Column::Id)
                        .update_columns([
                            store::Column::Name,
                            store::Column::LocationId,
                            store::Column::AddressStreet,
                            store::Column::AddressCity,
                            store::Column::AddressPostalCode,
                            store::Column::AddressCountry,
                            store::Column::Phone,
                            store::Column::GeoLatitude,
                            store::Column::GeoLongitude,
                            store::Column::Schedules,
                            store::Column::Services,
                            store::Column::Status,
                            store::Column::LogoObjectName,
                            store::Column::UpdatedAt,
                        ])
                        .to_owned(),
                )
                .exec(&*self.0).await.map_err(db_err)?;
        }
        Ok(())
    }
}

// ── PgRegisterRepository ──────────────────────────────────────────────────────

pub struct PgRegisterRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl RegisterRepository for PgRegisterRepository {
    async fn find_by_id(&self, id: RegisterId) -> Result<Option<Register>, AppError> {
        Ok(register::Entity::find_by_id(id.0)
            .one(&*self.0).await.map_err(db_err)?
            .map(model_to_register))
    }

    async fn name_exists_in_store(&self, store_id: StoreId, name: &str) -> Result<bool, AppError> {
        let count = register::Entity::find()
            .filter(
                Condition::all()
                    .add(register::Column::StoreId.eq(store_id.0))
                    .add(register::Column::Name.eq(name.to_string()))
            )
            .count(&*self.0).await.map_err(db_err)?;
        Ok(count > 0)
    }

    async fn get_by_store_id(&self, store_id: StoreId) -> Result<Vec<Register>, AppError> {
        Ok(register::Entity::find()
            .filter(register::Column::StoreId.eq(store_id.0))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(model_to_register).collect())
    }

    async fn get_paged(
        &self,
        store_id:  StoreId,
        page:      i32,
        page_size: i32,
    ) -> Result<PagedResult<Register>, AppError> {
        let q = register::Entity::find()
            .filter(register::Column::StoreId.eq(store_id.0));
        let offset = ((page - 1) * page_size) as u64;
        let limit  = page_size as u64;
        let total  = q.clone().count(&*self.0).await.map_err(db_err)? as i64;
        let items  = q.offset(offset).limit(limit).all(&*self.0).await.map_err(db_err)?
            .into_iter().map(model_to_register).collect();
        Ok(PagedResult::new(items, total, page, page_size))
    }

    async fn save(&self, reg: &mut Register) -> Result<(), AppError> {
        if reg.id.0 == 0 {
            let active = register::ActiveModel {
                id:              ActiveValue::NotSet,
                store_id:        Set(reg.store_id.0),
                name:            Set(reg.name.clone()),
                current_balance: Set(reg.current_balance),
                status:          Set(reg.status.as_str().to_string()),
                last_open:       Set(reg.last_open.map(from_utc)),
                last_close:      Set(reg.last_close.map(from_utc)),
                created_at:      Set(from_utc(reg.created_at)),
                updated_at:      Set(Some(from_utc(reg.updated_at))),
            };
            let result = register::Entity::insert(active).exec(&*self.0).await.map_err(db_err)?;
            reg.id = RegisterId(result.last_insert_id);
        } else {
            let active = register::ActiveModel {
                id:              Set(reg.id.0),
                store_id:        Set(reg.store_id.0),
                name:            Set(reg.name.clone()),
                current_balance: Set(reg.current_balance),
                status:          Set(reg.status.as_str().to_string()),
                last_open:       Set(reg.last_open.map(from_utc)),
                last_close:      Set(reg.last_close.map(from_utc)),
                created_at:      Set(from_utc(reg.created_at)),
                updated_at:      Set(Some(from_utc(reg.updated_at))),
            };
            register::Entity::insert(active)
                .on_conflict(
                    sea_orm::sea_query::OnConflict::column(register::Column::Id)
                        .update_columns([
                            register::Column::Name,
                            register::Column::CurrentBalance,
                            register::Column::Status,
                            register::Column::LastOpen,
                            register::Column::LastClose,
                            register::Column::UpdatedAt,
                        ])
                        .to_owned(),
                )
                .exec(&*self.0).await.map_err(db_err)?;
        }
        Ok(())
    }
}
