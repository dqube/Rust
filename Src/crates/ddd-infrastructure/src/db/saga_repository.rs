//! SeaORM implementation of [`ddd_shared_kernel::SagaInstanceRepository`].

use async_trait::async_trait;
use ddd_shared_kernel::saga::{
    SagaInstance, SagaInstanceRepository, SagaStatus, SagaStepState,
};
use ddd_shared_kernel::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use uuid::Uuid;

use crate::db::models::saga::{ActiveModel, Column, Entity, Model};

/// Postgres-backed saga instance repository.
pub struct SeaOrmSagaInstanceRepository {
    db: DatabaseConnection,
}

impl SeaOrmSagaInstanceRepository {
    /// Create a new repository.
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn to_domain(m: Model) -> AppResult<SagaInstance> {
        let status: SagaStatus =
            serde_json::from_value(serde_json::json!(m.status)).map_err(|e| {
                AppError::internal(format!("invalid saga status '{}': {e}", m.status))
            })?;

        let step_states: Vec<SagaStepState> = serde_json::from_value(m.step_states)
            .map_err(|e| AppError::internal(format!("invalid step_states JSON: {e}")))?;

        Ok(SagaInstance {
            id: m.id,
            saga_type: m.saga_type,
            status,
            current_step: m.current_step.max(0) as usize,
            payload: m.payload,
            step_states,
            created_at: m.created_at,
            updated_at: m.updated_at,
            version: m.version.max(0) as u64,
        })
    }

    fn to_active_model(s: &SagaInstance) -> AppResult<ActiveModel> {
        let step_states_json = serde_json::to_value(&s.step_states)
            .map_err(|e| AppError::internal(format!("failed to serialise step_states: {e}")))?;

        Ok(ActiveModel {
            id: Set(s.id),
            saga_type: Set(s.saga_type.clone()),
            status: Set(s.status.to_string()),
            current_step: Set(s.current_step as i32),
            payload: Set(s.payload.clone()),
            step_states: Set(step_states_json),
            created_at: Set(s.created_at),
            updated_at: Set(s.updated_at),
            version: Set(s.version as i64),
        })
    }
}

fn db_err(e: impl std::fmt::Display) -> AppError {
    AppError::database(e.to_string())
}

#[async_trait]
impl SagaInstanceRepository for SeaOrmSagaInstanceRepository {
    async fn save(&self, instance: &SagaInstance) -> AppResult<()> {
        let am = Self::to_active_model(instance)?;
        am.insert(&self.db).await.map_err(db_err)?;
        Ok(())
    }

    async fn update(&self, instance: &SagaInstance) -> AppResult<()> {
        // Optimistic concurrency: only update if version matches the
        // previous value (current - 1).
        let expected_version = instance
            .version
            .checked_sub(1)
            .ok_or_else(|| AppError::internal("version underflow"))?
            as i64;

        let am = Self::to_active_model(instance)?;
        let result = Entity::update(am)
            .filter(Column::Id.eq(instance.id))
            .filter(Column::Version.eq(expected_version))
            .exec(&self.db)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(sea_orm::DbErr::RecordNotUpdated) => Err(AppError::Conflict {
                message: format!(
                    "saga {} version conflict (expected {expected_version})",
                    instance.id
                ),
            }),
            Err(e) => Err(db_err(e)),
        }
    }

    async fn find_by_id(&self, id: Uuid) -> AppResult<SagaInstance> {
        let model = Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(db_err)?
            .ok_or_else(|| AppError::NotFound {
                resource: "SagaInstance".into(),
                id: id.to_string(),
            })?;
        Self::to_domain(model)
    }

    async fn find_by_status(&self, status: SagaStatus) -> AppResult<Vec<SagaInstance>> {
        let rows = Entity::find()
            .filter(Column::Status.eq(status.to_string()))
            .all(&self.db)
            .await
            .map_err(db_err)?;
        rows.into_iter().map(Self::to_domain).collect()
    }
}
