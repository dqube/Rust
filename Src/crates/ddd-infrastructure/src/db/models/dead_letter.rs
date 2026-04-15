//! SeaORM entity for the `dead_letter_messages` table.

use sea_orm::entity::prelude::*;

/// Row model — mirrors [`ddd_shared_kernel::DeadLetterMessage`].
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "dead_letter_messages")]
pub struct Model {
    /// Primary key.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// The id of the original outbox / inbox message.
    pub original_message_id: Uuid,
    /// `"outbox"` or `"inbox"`.
    pub origin: String,
    /// Stable event-type string.
    pub event_type: String,
    /// Broker routing key.
    pub subject: String,
    /// JSON payload.
    #[sea_orm(column_type = "Json")]
    pub payload: Json,
    /// Total number of attempts before dead-lettering.
    pub attempts: i32,
    /// The last error recorded.
    pub last_error: String,
    /// When the original message was created / received.
    pub original_created_at: DateTimeUtc,
    /// When the message was moved to the dead-letter store.
    pub dead_lettered_at: DateTimeUtc,
}

/// No relations for the dead-letter entity.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
