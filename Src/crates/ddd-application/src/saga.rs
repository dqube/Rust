//! Saga orchestrator — drives multi-step distributed transactions.
//!
//! The [`DefaultSagaOrchestrator`] is the concrete implementation of the
//! [`SagaOrchestrator`] trait defined in `ddd-shared-kernel`.  It persists
//! saga state via [`SagaInstanceRepository`] and emits step commands through
//! [`OutboxRepository`], so every state transition is transactional.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use ddd_shared_kernel::{
    AppError, AppResult, OutboxMessage, OutboxRepository, SagaDefinition, SagaInstance,
    SagaInstanceRepository, SagaOrchestrator, SagaStatus, SagaStepStatus,
};

// ─── SagaDefinitionRegistry ──────────────────────────────────────────────────

/// Holds all registered saga definitions so the orchestrator can look them up
/// by `saga_type`.
#[derive(Debug, Default, Clone)]
pub struct SagaDefinitionRegistry {
    definitions: HashMap<String, SagaDefinition>,
}

impl SagaDefinitionRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a saga definition.
    pub fn register(&mut self, definition: SagaDefinition) {
        self.definitions
            .insert(definition.saga_type.clone(), definition);
    }

    /// Look up a definition by saga type.
    pub fn get(&self, saga_type: &str) -> Option<&SagaDefinition> {
        self.definitions.get(saga_type)
    }
}

// ─── DefaultSagaOrchestrator ─────────────────────────────────────────────────

/// Drives sagas forward step by step and compensates on failure.
///
/// # Flow
///
/// 1. `start()` creates a [`SagaInstance`], persists it, and writes the first
///    step command to the outbox.
/// 2. When a participant service completes a step, it publishes a reply that
///    the inbox delivers to `on_step_completed()`.  The orchestrator advances
///    to the next step (or marks the saga completed).
/// 3. On failure, `on_step_failed()` begins compensating in reverse order.
/// 4. `on_compensation_completed()` walks backwards until all completed steps
///    are compensated.
pub struct DefaultSagaOrchestrator {
    saga_repo: Arc<dyn SagaInstanceRepository>,
    outbox_repo: Arc<dyn OutboxRepository>,
    registry: SagaDefinitionRegistry,
}

impl DefaultSagaOrchestrator {
    /// Create a new orchestrator.
    pub fn new(
        saga_repo: Arc<dyn SagaInstanceRepository>,
        outbox_repo: Arc<dyn OutboxRepository>,
        registry: SagaDefinitionRegistry,
    ) -> Self {
        Self {
            saga_repo,
            outbox_repo,
            registry,
        }
    }

    /// Look up the definition or return an error.
    fn definition(&self, saga_type: &str) -> AppResult<&SagaDefinition> {
        self.registry.get(saga_type).ok_or_else(|| {
            AppError::internal(format!("unknown saga type: {saga_type}"))
        })
    }

    /// Emit a step action command to the outbox.
    async fn emit_action(
        &self,
        saga: &SagaInstance,
        def: &SagaDefinition,
        step_index: usize,
    ) -> AppResult<()> {
        let step = &def.steps[step_index];
        let msg = OutboxMessage::new(
            saga.id.to_string(),
            &def.saga_type,
            &step.action_event_type,
            &step.action_subject,
            serde_json::json!({
                "saga_id": saga.id,
                "step_index": step_index,
                "payload": saga.payload,
            }),
        );
        self.outbox_repo.save(&msg).await
    }

    /// Emit a compensation command to the outbox.
    async fn emit_compensation(
        &self,
        saga: &SagaInstance,
        def: &SagaDefinition,
        step_index: usize,
    ) -> AppResult<()> {
        let step = &def.steps[step_index];
        let (event_type, subject) = match (
            step.compensation_event_type.as_ref(),
            step.compensation_subject.as_ref(),
        ) {
            (Some(et), Some(s)) => (et, s),
            _ => {
                // No compensation defined — skip.
                return Ok(());
            }
        };

        let msg = OutboxMessage::new(
            saga.id.to_string(),
            &def.saga_type,
            event_type,
            subject,
            serde_json::json!({
                "saga_id": saga.id,
                "step_index": step_index,
                "payload": saga.payload,
                "step_response": saga.step_states[step_index].response,
            }),
        );
        self.outbox_repo.save(&msg).await
    }

    /// Find the next step to compensate (walking backwards from
    /// `start_index`).  Returns `None` when all steps are compensated or
    /// pending.
    fn next_compensation_index(saga: &SagaInstance, start_index: usize) -> Option<usize> {
        (0..=start_index)
            .rev()
            .find(|&i| saga.step_states[i].status == SagaStepStatus::Completed)
    }
}

#[async_trait]
impl SagaOrchestrator for DefaultSagaOrchestrator {
    async fn start(&self, saga_type: &str, payload: Value) -> AppResult<Uuid> {
        let def = self.definition(saga_type)?;
        if def.steps.is_empty() {
            return Err(AppError::internal("saga definition has no steps"));
        }

        let mut saga = SagaInstance::new(saga_type, def.steps.len(), payload);
        saga.status = SagaStatus::Executing;
        saga.step_states[0].status = SagaStepStatus::Executing;
        saga.version = 1;

        #[cfg(feature = "tracing")]
        tracing::info!(
            saga_id = %saga.id,
            saga_type = saga_type,
            steps = def.steps.len(),
            "Starting saga"
        );

        self.saga_repo.save(&saga).await?;
        self.emit_action(&saga, def, 0).await?;

        Ok(saga.id)
    }

    async fn on_step_completed(
        &self,
        saga_id: Uuid,
        step_index: usize,
        response: Value,
    ) -> AppResult<()> {
        let mut saga = self.saga_repo.find_by_id(saga_id).await?;
        let def = self.definition(&saga.saga_type)?;

        // Validate state.
        if step_index >= saga.step_states.len() {
            return Err(AppError::internal(format!(
                "step index {step_index} out of range for saga {}",
                saga.id
            )));
        }
        if saga.step_states[step_index].status != SagaStepStatus::Executing {
            return Err(AppError::Conflict {
                message: format!(
                    "step {step_index} is not executing (status: {})",
                    saga.step_states[step_index].status
                ),
            });
        }

        saga.step_states[step_index].status = SagaStepStatus::Completed;
        saga.step_states[step_index].response = Some(response);
        saga.updated_at = chrono::Utc::now();
        saga.version += 1;

        let next = step_index + 1;
        if next < def.steps.len() {
            // Advance to the next step.
            saga.current_step = next;
            saga.step_states[next].status = SagaStepStatus::Executing;

            #[cfg(feature = "tracing")]
            tracing::info!(
                saga_id = %saga.id,
                step = next,
                "Saga advancing to next step"
            );

            self.saga_repo.update(&saga).await?;
            self.emit_action(&saga, def, next).await?;
        } else {
            // All steps done — saga completed.
            saga.status = SagaStatus::Completed;

            #[cfg(feature = "tracing")]
            tracing::info!(saga_id = %saga.id, "Saga completed successfully");

            self.saga_repo.update(&saga).await?;
        }

        Ok(())
    }

    async fn on_step_failed(
        &self,
        saga_id: Uuid,
        step_index: usize,
        error: String,
    ) -> AppResult<()> {
        let mut saga = self.saga_repo.find_by_id(saga_id).await?;
        let def = self.definition(&saga.saga_type)?;

        if step_index >= saga.step_states.len() {
            return Err(AppError::internal(format!(
                "step index {step_index} out of range for saga {}",
                saga.id
            )));
        }

        saga.step_states[step_index].status = SagaStepStatus::Failed;
        saga.step_states[step_index].error = Some(error.clone());
        saga.updated_at = chrono::Utc::now();
        saga.version += 1;

        #[cfg(feature = "tracing")]
        tracing::warn!(
            saga_id = %saga.id,
            step = step_index,
            error = %error,
            "Saga step failed, starting compensation"
        );

        // Begin compensation from the step before the failed one.
        if step_index == 0 {
            // Nothing to compensate — the first step itself failed.
            saga.status = SagaStatus::CompensationCompleted;
            self.saga_repo.update(&saga).await?;
            return Ok(());
        }

        match Self::next_compensation_index(&saga, step_index - 1) {
            Some(comp_idx) => {
                saga.status = SagaStatus::Compensating;
                saga.current_step = comp_idx;
                saga.step_states[comp_idx].status = SagaStepStatus::CompensatingStep;

                self.saga_repo.update(&saga).await?;
                self.emit_compensation(&saga, def, comp_idx).await?;
            }
            None => {
                // No completed steps to compensate.
                saga.status = SagaStatus::CompensationCompleted;
                self.saga_repo.update(&saga).await?;
            }
        }

        Ok(())
    }

    async fn on_compensation_completed(
        &self,
        saga_id: Uuid,
        step_index: usize,
    ) -> AppResult<()> {
        let mut saga = self.saga_repo.find_by_id(saga_id).await?;
        let def = self.definition(&saga.saga_type)?;

        if step_index >= saga.step_states.len() {
            return Err(AppError::internal(format!(
                "step index {step_index} out of range for saga {}",
                saga.id
            )));
        }

        saga.step_states[step_index].status = SagaStepStatus::Compensated;
        saga.updated_at = chrono::Utc::now();
        saga.version += 1;

        // Find the next step to compensate (walking backwards).
        if step_index == 0 {
            saga.status = SagaStatus::CompensationCompleted;

            #[cfg(feature = "tracing")]
            tracing::info!(saga_id = %saga.id, "Saga compensation completed");

            self.saga_repo.update(&saga).await?;
            return Ok(());
        }

        match Self::next_compensation_index(&saga, step_index - 1) {
            Some(comp_idx) => {
                saga.current_step = comp_idx;
                saga.step_states[comp_idx].status = SagaStepStatus::CompensatingStep;

                #[cfg(feature = "tracing")]
                tracing::info!(
                    saga_id = %saga.id,
                    step = comp_idx,
                    "Compensating next step"
                );

                self.saga_repo.update(&saga).await?;
                self.emit_compensation(&saga, def, comp_idx).await?;
            }
            None => {
                saga.status = SagaStatus::CompensationCompleted;

                #[cfg(feature = "tracing")]
                tracing::info!(saga_id = %saga.id, "Saga compensation completed");

                self.saga_repo.update(&saga).await?;
            }
        }

        Ok(())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ddd_shared_kernel::SagaStepDefinition;
    use std::sync::Mutex;

    // Minimal in-memory SagaInstanceRepository for tests.
    struct InMemorySagaRepo {
        instances: Mutex<HashMap<Uuid, SagaInstance>>,
    }

    impl InMemorySagaRepo {
        fn new() -> Self {
            Self {
                instances: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl SagaInstanceRepository for InMemorySagaRepo {
        async fn save(&self, instance: &SagaInstance) -> AppResult<()> {
            self.instances
                .lock()
                .unwrap()
                .insert(instance.id, instance.clone());
            Ok(())
        }

        async fn update(&self, instance: &SagaInstance) -> AppResult<()> {
            let mut store = self.instances.lock().unwrap();
            let existing = store.get(&instance.id).ok_or_else(|| {
                AppError::NotFound {
                    resource: "SagaInstance".into(),
                    id: instance.id.to_string(),
                }
            })?;
            if existing.version >= instance.version {
                return Err(AppError::Conflict {
                    message: "version mismatch".into(),
                });
            }
            store.insert(instance.id, instance.clone());
            Ok(())
        }

        async fn find_by_id(&self, id: Uuid) -> AppResult<SagaInstance> {
            self.instances
                .lock()
                .unwrap()
                .get(&id)
                .cloned()
                .ok_or_else(|| AppError::NotFound {
                    resource: "SagaInstance".into(),
                    id: id.to_string(),
                })
        }

        async fn find_by_status(&self, status: SagaStatus) -> AppResult<Vec<SagaInstance>> {
            Ok(self
                .instances
                .lock()
                .unwrap()
                .values()
                .filter(|s| s.status == status)
                .cloned()
                .collect())
        }
    }

    // Minimal in-memory OutboxRepository — just records saved messages.
    struct InMemoryOutboxRepo {
        messages: Mutex<Vec<OutboxMessage>>,
    }

    impl InMemoryOutboxRepo {
        fn new() -> Self {
            Self {
                messages: Mutex::new(Vec::new()),
            }
        }

        fn count(&self) -> usize {
            self.messages.lock().unwrap().len()
        }
    }

    #[async_trait]
    impl OutboxRepository for InMemoryOutboxRepo {
        async fn save(&self, message: &OutboxMessage) -> AppResult<()> {
            self.messages.lock().unwrap().push(message.clone());
            Ok(())
        }

        async fn mark_as_published(&self, _id: Uuid) -> AppResult<()> {
            Ok(())
        }

        async fn mark_as_failed(&self, _id: Uuid, _error: &str) -> AppResult<()> {
            Ok(())
        }

        async fn find_unpublished(&self, _limit: u32) -> AppResult<Vec<OutboxMessage>> {
            Ok(vec![])
        }

        async fn delete_published_older_than(
            &self,
            _older_than: chrono::DateTime<chrono::Utc>,
        ) -> AppResult<u64> {
            Ok(0)
        }
    }

    fn create_test_definition() -> SagaDefinition {
        SagaDefinition {
            saga_type: "create_order".into(),
            steps: vec![
                SagaStepDefinition {
                    name: "reserve_inventory".into(),
                    action_event_type: "inventory.reserve.v1".into(),
                    action_subject: "inventory.commands.reserve".into(),
                    compensation_event_type: Some("inventory.release.v1".into()),
                    compensation_subject: Some("inventory.commands.release".into()),
                },
                SagaStepDefinition {
                    name: "process_payment".into(),
                    action_event_type: "payment.charge.v1".into(),
                    action_subject: "payment.commands.charge".into(),
                    compensation_event_type: Some("payment.refund.v1".into()),
                    compensation_subject: Some("payment.commands.refund".into()),
                },
                SagaStepDefinition {
                    name: "confirm_order".into(),
                    action_event_type: "order.confirm.v1".into(),
                    action_subject: "order.commands.confirm".into(),
                    compensation_event_type: None,
                    compensation_subject: None,
                },
            ],
        }
    }

    fn create_orchestrator(
        saga_repo: Arc<InMemorySagaRepo>,
        outbox_repo: Arc<InMemoryOutboxRepo>,
    ) -> DefaultSagaOrchestrator {
        let mut registry = SagaDefinitionRegistry::new();
        registry.register(create_test_definition());
        DefaultSagaOrchestrator::new(saga_repo, outbox_repo, registry)
    }

    #[tokio::test]
    async fn start_saga_emits_first_step() {
        let saga_repo = Arc::new(InMemorySagaRepo::new());
        let outbox_repo = Arc::new(InMemoryOutboxRepo::new());
        let orch = create_orchestrator(saga_repo.clone(), outbox_repo.clone());

        let saga_id = orch
            .start("create_order", serde_json::json!({"order_id": "123"}))
            .await
            .unwrap();

        // One outbox message for the first step.
        assert_eq!(outbox_repo.count(), 1);

        let saga = saga_repo.find_by_id(saga_id).await.unwrap();
        assert_eq!(saga.status, SagaStatus::Executing);
        assert_eq!(saga.current_step, 0);
        assert_eq!(saga.step_states[0].status, SagaStepStatus::Executing);
    }

    #[tokio::test]
    async fn happy_path_completes_saga() {
        let saga_repo = Arc::new(InMemorySagaRepo::new());
        let outbox_repo = Arc::new(InMemoryOutboxRepo::new());
        let orch = create_orchestrator(saga_repo.clone(), outbox_repo.clone());

        let saga_id = orch
            .start("create_order", serde_json::json!({"order_id": "123"}))
            .await
            .unwrap();

        // Complete step 0.
        orch.on_step_completed(saga_id, 0, serde_json::json!({"reserved": true}))
            .await
            .unwrap();

        // Complete step 1.
        orch.on_step_completed(saga_id, 1, serde_json::json!({"charged": true}))
            .await
            .unwrap();

        // Complete step 2.
        orch.on_step_completed(saga_id, 2, serde_json::json!({"confirmed": true}))
            .await
            .unwrap();

        let saga = saga_repo.find_by_id(saga_id).await.unwrap();
        assert_eq!(saga.status, SagaStatus::Completed);
        // start + 3 steps = 4 outbox messages.
        assert_eq!(outbox_repo.count(), 3);
    }

    #[tokio::test]
    async fn step_failure_triggers_compensation() {
        let saga_repo = Arc::new(InMemorySagaRepo::new());
        let outbox_repo = Arc::new(InMemoryOutboxRepo::new());
        let orch = create_orchestrator(saga_repo.clone(), outbox_repo.clone());

        let saga_id = orch
            .start("create_order", serde_json::json!({"order_id": "123"}))
            .await
            .unwrap();

        // Complete step 0.
        orch.on_step_completed(saga_id, 0, serde_json::json!({"reserved": true}))
            .await
            .unwrap();

        // Step 1 fails.
        orch.on_step_failed(saga_id, 1, "payment declined".into())
            .await
            .unwrap();

        let saga = saga_repo.find_by_id(saga_id).await.unwrap();
        assert_eq!(saga.status, SagaStatus::Compensating);
        assert_eq!(saga.step_states[1].status, SagaStepStatus::Failed);
        assert_eq!(saga.step_states[0].status, SagaStepStatus::CompensatingStep);

        // Compensation for step 0 completes.
        orch.on_compensation_completed(saga_id, 0).await.unwrap();

        let saga = saga_repo.find_by_id(saga_id).await.unwrap();
        assert_eq!(saga.status, SagaStatus::CompensationCompleted);
        assert_eq!(saga.step_states[0].status, SagaStepStatus::Compensated);
    }

    #[tokio::test]
    async fn first_step_failure_no_compensation_needed() {
        let saga_repo = Arc::new(InMemorySagaRepo::new());
        let outbox_repo = Arc::new(InMemoryOutboxRepo::new());
        let orch = create_orchestrator(saga_repo.clone(), outbox_repo.clone());

        let saga_id = orch
            .start("create_order", serde_json::json!({"order_id": "123"}))
            .await
            .unwrap();

        orch.on_step_failed(saga_id, 0, "inventory unavailable".into())
            .await
            .unwrap();

        let saga = saga_repo.find_by_id(saga_id).await.unwrap();
        // No completed steps to compensate → immediately compensation-completed.
        assert_eq!(saga.status, SagaStatus::CompensationCompleted);
    }
}
