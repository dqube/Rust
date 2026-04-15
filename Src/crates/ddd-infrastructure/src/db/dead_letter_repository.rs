//! SeaORM implementation of [`ddd_shared_kernel::DeadLetterRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ddd_shared_kernel::dead_letter::{DeadLetterMessage, DeadLetterOrigin, DeadLetterRepository};
use ddd_shared_kernel::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect,
};

use crate::db::models::dead_letter::{ActiveModel, Column, Entity, Model};

/// Postgres-backed dead-letter repository.
pub struct SeaOrmDeadLetterRepository {
    db: DatabaseConnection,
}

impl SeaOrmDeadLetterRepository {
    /// Create a new repository.
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn to_domain(m: Model) -> DeadLetterMessage {
        let origin = match m.origin.as_str() {
            "inbox" => DeadLetterOrigin::Inbox,
            _ => DeadLetterOrigin::Outbox,
        };
        DeadLetterMessage {
            id: m.id,
            original_message_id: m.original_message_id,
            origin,
            event_type: m.event_type,
            subject: m.subject,
            payload: m.payload,
            attempts: m.attempts.max(0) as u32,
            last_error: m.last_error,
            original_created_at: m.original_created_at,
            dead_lettered_at: m.dead_lettered_at,
        }
    }
}

fn db_err(e: impl std::fmt::Display) -> AppError {
    AppError::database(e.to_string())
}

#[async_trait]
impl DeadLetterRepository for SeaOrmDeadLetterRepository {
    async fn save(&self, message: &DeadLetterMessage) -> AppResult<()> {
        let am = ActiveModel {
            id: Set(message.id),
            original_message_id: Set(message.original_message_id),
            origin: Set(message.origin.to_string()),
            event_type: Set(message.event_type.clone()),
            subject: Set(message.subject.clone()),
            payload: Set(message.payload.clone()),
            attempts: Set(message.attempts as i32),
            last_error: Set(message.last_error.clone()),
            original_created_at: Set(message.original_created_at),
            dead_lettered_at: Set(message.dead_lettered_at),
        };
        am.insert(&self.db).await.map_err(db_err)?;
        Ok(())
    }

    async fn find_by_origin(
        &self,
        origin: DeadLetterOrigin,
        limit: u32,
    ) -> AppResult<Vec<DeadLetterMessage>> {
        let rows = Entity::find()
            .filter(Column::Origin.eq(origin.to_string()))
            .order_by_desc(Column::DeadLetteredAt)
            .limit(u64::from(limit))
            .all(&self.db)
            .await
            .map_err(db_err)?;
        Ok(rows.into_iter().map(Self::to_domain).collect())
    }

    async fn delete_older_than(&self, older_than: DateTime<Utc>) -> AppResult<u64> {
        let res = Entity::delete_many()
            .filter(Column::DeadLetteredAt.lt(older_than))
            .exec(&self.db)
            .await
            .map_err(db_err)?;
        Ok(res.rows_affected)
    }
}
