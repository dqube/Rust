//! Database-backed [`IdempotencyStore`] implementation.
//!
//! Uses a `idempotency_keys` table with atomic `INSERT … ON CONFLICT DO
//! NOTHING` semantics to guarantee exactly-once acquisition.

use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use ddd_shared_kernel::idempotency::{IdempotencyRecord, IdempotencyStore};
use ddd_shared_kernel::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter,
};
use serde_json::Value;

use crate::db::models::idempotency::{ActiveModel, Column, Entity};

/// Postgres-backed idempotency store.
///
/// Relies on a unique constraint on the `key` column. The `try_acquire` method
/// uses `INSERT … ON CONFLICT DO NOTHING` to atomically claim a key.
pub struct DbIdempotencyStore {
    db: DatabaseConnection,
}

impl DbIdempotencyStore {
    /// Create a new store backed by the given connection.
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn db_err(e: impl std::fmt::Display) -> AppError {
    AppError::database(e.to_string())
}

#[async_trait]
impl IdempotencyStore for DbIdempotencyStore {
    async fn try_acquire(&self, key: &str, ttl: Duration) -> AppResult<bool> {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::from_std(ttl).unwrap_or(chrono::Duration::hours(24));

        // First, clean up any expired key to allow reuse.
        Entity::delete_many()
            .filter(Column::Key.eq(key))
            .filter(Column::ExpiresAt.lt(now))
            .exec(&self.db)
            .await
            .map_err(db_err)?;

        let am = ActiveModel {
            key: Set(key.to_owned()),
            response: Set(None),
            created_at: Set(now),
            expires_at: Set(expires_at),
        };

        match am.insert(&self.db).await {
            Ok(_) => Ok(true),
            Err(DbErr::RecordNotInserted) => Ok(false),
            Err(e) => {
                let s = e.to_string();
                if s.contains("duplicate")
                    || s.contains("UNIQUE")
                    || s.contains("unique")
                    || s.contains("23505") // Postgres unique_violation
                {
                    Ok(false)
                } else {
                    Err(db_err(e))
                }
            }
        }
    }

    async fn store_response(&self, key: &str, response: &Value) -> AppResult<()> {
        let found = Entity::find_by_id(key.to_owned())
            .one(&self.db)
            .await
            .map_err(db_err)?;
        let Some(m) = found else {
            return Ok(());
        };
        let mut am: ActiveModel = m.into();
        am.response = Set(Some(response.clone()));
        am.update(&self.db).await.map_err(db_err)?;
        Ok(())
    }

    async fn get_response(&self, key: &str) -> AppResult<Option<IdempotencyRecord>> {
        let found = Entity::find_by_id(key.to_owned())
            .one(&self.db)
            .await
            .map_err(db_err)?;
        match found {
            Some(m) if m.response.is_some() => Ok(Some(IdempotencyRecord {
                key: m.key,
                response: m.response.unwrap(),
            })),
            _ => Ok(None),
        }
    }

    async fn release(&self, key: &str) -> AppResult<()> {
        Entity::delete_by_id(key.to_owned())
            .exec(&self.db)
            .await
            .map_err(db_err)?;
        Ok(())
    }
}
