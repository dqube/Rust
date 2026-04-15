//! SeaORM entity for the `inbox_messages` table.

use sea_orm::entity::prelude::*;

/// Row model — mirrors [`ddd_shared_kernel::InboxMessage`].
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "inbox_messages")]
pub struct Model {
    /// Primary key (broker-assigned idempotency key).
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Stable event-type string.
    pub event_type: String,
    /// Broker routing key.
    pub subject: String,
    /// JSON payload.
    #[sea_orm(column_type = "Json")]
    pub payload: Json,
    /// Source service that published the event.
    pub source: String,
    /// When the message was received.
    pub received_at: DateTimeUtc,
    /// When the message was processed.
    pub processed_at: Option<DateTimeUtc>,
    /// Number of processing attempts.
    pub attempts: i32,
    /// Last error message.
    pub last_error: Option<String>,
}

/// No relations for the inbox entity.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
