//! SeaORM implementation of [`ddd_shared_kernel::InboxRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ddd_shared_kernel::{AppError, AppResult, InboxMessage, InboxRepository};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect,
};
use uuid::Uuid;

use crate::db::models::inbox::{ActiveModel, Column, Entity, Model};

/// Postgres-backed inbox repository.
pub struct SeaOrmInboxRepository {
    db: DatabaseConnection,
}

impl SeaOrmInboxRepository {
    /// Create a new repository.
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn to_domain(m: Model) -> InboxMessage {
        InboxMessage {
            id: m.id,
            event_type: m.event_type,
            subject: m.subject,
            payload: m.payload,
            source: m.source,
            received_at: m.received_at,
            processed_at: m.processed_at,
            attempts: m.attempts.max(0) as u32,
            last_error: m.last_error,
        }
    }
}

fn db_err(e: impl std::fmt::Display) -> AppError {
    AppError::database(e.to_string())
}

#[async_trait]
impl InboxRepository for SeaOrmInboxRepository {
    async fn save(&self, message: &InboxMessage) -> AppResult<bool> {
        let am = ActiveModel {
            id: Set(message.id),
            event_type: Set(message.event_type.clone()),
            subject: Set(message.subject.clone()),
            payload: Set(message.payload.clone()),
            source: Set(message.source.clone()),
            received_at: Set(message.received_at),
            processed_at: Set(message.processed_at),
            attempts: Set(message.attempts as i32),
            last_error: Set(message.last_error.clone()),
        };
        match am.insert(&self.db).await {
            Ok(_) => Ok(true),
            Err(DbErr::RecordNotInserted) => Ok(false),
            Err(e) => {
                let s = e.to_string();
                if s.contains("duplicate") || s.contains("UNIQUE") || s.contains("unique") {
                    Ok(false)
                } else {
                    Err(db_err(e))
                }
            }
        }
    }

    async fn mark_as_processed(&self, id: Uuid) -> AppResult<()> {
        let found = Entity::find_by_id(id).one(&self.db).await.map_err(db_err)?;
        let Some(m) = found else { return Ok(()); };
        let mut am: ActiveModel = m.into();
        am.processed_at = Set(Some(Utc::now()));
        am.last_error = Set(None);
        am.update(&self.db).await.map_err(db_err)?;
        Ok(())
    }

    async fn mark_as_failed(&self, id: Uuid, error: &str) -> AppResult<()> {
        let found = Entity::find_by_id(id).one(&self.db).await.map_err(db_err)?;
        let Some(m) = found else { return Ok(()); };
        let new_attempts = m.attempts + 1;
        let mut am: ActiveModel = m.into();
        am.attempts = Set(new_attempts);
        am.last_error = Set(Some(error.to_owned()));
        am.update(&self.db).await.map_err(db_err)?;
        Ok(())
    }

    async fn find_unprocessed(&self, limit: u32) -> AppResult<Vec<InboxMessage>> {
        let rows = Entity::find()
            .filter(Column::ProcessedAt.is_null())
            .order_by_asc(Column::ReceivedAt)
            .limit(u64::from(limit))
            .all(&self.db)
            .await
            .map_err(db_err)?;
        Ok(rows.into_iter().map(Self::to_domain).collect())
    }

    async fn delete_processed_older_than(&self, older_than: DateTime<Utc>) -> AppResult<u64> {
        let res = Entity::delete_many()
            .filter(Column::ProcessedAt.is_not_null())
            .filter(Column::ProcessedAt.lt(older_than))
            .exec(&self.db)
            .await
            .map_err(db_err)?;
        Ok(res.rows_affected)
    }
}
