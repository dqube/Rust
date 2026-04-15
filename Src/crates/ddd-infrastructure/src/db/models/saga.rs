//! SeaORM entity for the `saga_instances` table.

use sea_orm::entity::prelude::*;

/// Row model — mirrors [`ddd_shared_kernel::SagaInstance`].
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "saga_instances")]
pub struct Model {
    /// Primary key — saga execution id.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Saga type identifier (e.g. `"create_order"`).
    pub saga_type: String,
    /// Overall status as a string (e.g. `"executing"`, `"compensating"`).
    pub status: String,
    /// Zero-based index of the current step.
    pub current_step: i32,
    /// Initial saga payload.
    #[sea_orm(column_type = "Json")]
    pub payload: Json,
    /// JSON array of per-step state.
    #[sea_orm(column_type = "Json")]
    pub step_states: Json,
    /// When the saga was created.
    pub created_at: DateTimeUtc,
    /// When the saga last changed state.
    pub updated_at: DateTimeUtc,
    /// Optimistic concurrency version.
    pub version: i64,
}

/// No relations for the saga entity.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
