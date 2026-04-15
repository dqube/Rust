//! SeaORM entity for the `idempotency_keys` table.

use sea_orm::entity::prelude::*;

/// Row model — stores an idempotency key and its cached response.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "idempotency_keys")]
pub struct Model {
    /// The idempotency key (primary key).
    #[sea_orm(primary_key, auto_increment = false)]
    pub key: String,
    /// Serialised response (`null` while the command is in flight).
    #[sea_orm(column_type = "Json", nullable)]
    pub response: Option<Json>,
    /// When the key was first acquired.
    pub created_at: DateTimeUtc,
    /// When the key expires and may be reused.
    pub expires_at: DateTimeUtc,
}

/// No relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
