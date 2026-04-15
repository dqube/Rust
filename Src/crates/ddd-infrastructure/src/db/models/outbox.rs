//! SeaORM entity for the `outbox_messages` table.

use sea_orm::entity::prelude::*;

/// Row model — mirrors [`ddd_shared_kernel::OutboxMessage`].
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "outbox_messages")]
pub struct Model {
    /// Primary key.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Stringified aggregate id.
    pub aggregate_id: String,
    /// Aggregate type name.
    pub aggregate_type: String,
    /// Stable event-type string.
    pub event_type: String,
    /// Broker routing key.
    pub subject: String,
    /// JSON payload.
    #[sea_orm(column_type = "Json")]
    pub payload: Json,
    /// Creation timestamp.
    pub created_at: DateTimeUtc,
    /// When the message was published (null until then).
    pub published_at: Option<DateTimeUtc>,
    /// Number of publish attempts.
    pub attempts: i32,
    /// Last error message.
    pub last_error: Option<String>,
}

/// No relations for the outbox entity.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
