//! Saga orchestrator building blocks.
//!
//! A saga coordinates a multi-step distributed transaction across aggregates
//! or services.  Each step has an *action* and a *compensation* (rollback).
//! The orchestrator drives execution forward step by step; if any step fails
//! it runs compensations in reverse order.
//!
//! Step commands and responses travel through the outbox / inbox so that every
//! state transition is persisted inside a database transaction.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::AppResult;

// ─── SagaStatus ──────────────────────────────────────────────────────────────

/// The overall lifecycle state of a saga instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SagaStatus {
    /// The saga was created but the first step has not started yet.
    Started,
    /// A step action is currently executing.
    Executing,
    /// A step failed and compensations are running in reverse.
    Compensating,
    /// All steps completed successfully.
    Completed,
    /// All compensations finished after a failure.
    CompensationCompleted,
    /// The saga is in an unrecoverable state (e.g. a compensation failed).
    Failed,
}

impl std::fmt::Display for SagaStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Started => write!(f, "started"),
            Self::Executing => write!(f, "executing"),
            Self::Compensating => write!(f, "compensating"),
            Self::Completed => write!(f, "completed"),
            Self::CompensationCompleted => write!(f, "compensation_completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

// ─── SagaStepStatus ──────────────────────────────────────────────────────────

/// The lifecycle state of a single saga step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SagaStepStatus {
    /// Not yet started.
    Pending,
    /// The action command has been sent.
    Executing,
    /// The action completed successfully.
    Completed,
    /// The compensation command has been sent.
    CompensatingStep,
    /// The compensation completed successfully.
    Compensated,
    /// The action or compensation failed terminally.
    Failed,
}

impl std::fmt::Display for SagaStepStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Executing => write!(f, "executing"),
            Self::Completed => write!(f, "completed"),
            Self::CompensatingStep => write!(f, "compensating"),
            Self::Compensated => write!(f, "compensated"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

// ─── SagaStepDefinition ──────────────────────────────────────────────────────

/// Defines a single step in a saga with its action and compensation event
/// types.
#[derive(Debug, Clone)]
pub struct SagaStepDefinition {
    /// Human-readable name for this step (e.g. `"reserve_inventory"`).
    pub name: String,
    /// The event type string to publish when executing the action
    /// (e.g. `"inventory.reserve.v1"`).
    pub action_event_type: String,
    /// The broker subject for the action command.
    pub action_subject: String,
    /// The event type string to publish when compensating
    /// (e.g. `"inventory.release.v1"`).  `None` for the first step if no
    /// rollback is needed.
    pub compensation_event_type: Option<String>,
    /// The broker subject for the compensation command.
    pub compensation_subject: Option<String>,
}

// ─── SagaDefinition ──────────────────────────────────────────────────────────

/// A named, ordered list of steps that together form a saga type.
#[derive(Debug, Clone)]
pub struct SagaDefinition {
    /// Stable saga type identifier (e.g. `"create_order"`).
    pub saga_type: String,
    /// Ordered steps executed left-to-right; compensated right-to-left.
    pub steps: Vec<SagaStepDefinition>,
}

// ─── SagaStepState ───────────────────────────────────────────────────────────

/// Runtime state of a single step inside a [`SagaInstance`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStepState {
    /// Index into the parent definition's `steps` vector.
    pub step_index: usize,
    /// Current status.
    pub status: SagaStepStatus,
    /// Response payload returned by the action (if completed).
    pub response: Option<Value>,
    /// Error message when the step failed.
    pub error: Option<String>,
}

// ─── SagaInstance ─────────────────────────────────────────────────────────────

/// Runtime state of a single saga execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaInstance {
    /// Unique saga execution id.
    pub id: Uuid,
    /// Saga type identifier — corresponds to [`SagaDefinition::saga_type`].
    pub saga_type: String,
    /// Overall status.
    pub status: SagaStatus,
    /// Zero-based index of the step currently being executed or compensated.
    pub current_step: usize,
    /// The initial payload that triggered the saga.
    pub payload: Value,
    /// Per-step runtime state.
    pub step_states: Vec<SagaStepState>,
    /// When the saga was started.
    pub created_at: DateTime<Utc>,
    /// When the saga last changed state.
    pub updated_at: DateTime<Utc>,
    /// Version for optimistic concurrency (incremented each update).
    pub version: u64,
}

impl SagaInstance {
    /// Create a new saga instance with all steps in `Pending`.
    pub fn new(saga_type: impl Into<String>, step_count: usize, payload: Value) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            saga_type: saga_type.into(),
            status: SagaStatus::Started,
            current_step: 0,
            payload,
            step_states: (0..step_count)
                .map(|i| SagaStepState {
                    step_index: i,
                    status: SagaStepStatus::Pending,
                    response: None,
                    error: None,
                })
                .collect(),
            created_at: now,
            updated_at: now,
            version: 0,
        }
    }
}

// ─── SagaInstanceRepository ──────────────────────────────────────────────────

/// Persistence interface for saga instances.
#[async_trait]
pub trait SagaInstanceRepository: Send + Sync {
    /// Persist a new saga instance.
    ///
    /// # Errors
    /// Returns a database error when the insert fails.
    async fn save(&self, instance: &SagaInstance) -> AppResult<()>;

    /// Update an existing instance.  Implementations should check
    /// [`SagaInstance::version`] for optimistic concurrency.
    ///
    /// # Errors
    /// Returns [`AppError::Conflict`] on version mismatch, or a database error.
    async fn update(&self, instance: &SagaInstance) -> AppResult<()>;

    /// Load an instance by id.
    ///
    /// # Errors
    /// Returns [`AppError::NotFound`] when the id does not exist.
    async fn find_by_id(&self, id: Uuid) -> AppResult<SagaInstance>;

    /// Find all instances with the given status (e.g. for retry / monitoring).
    ///
    /// # Errors
    /// Returns a database error on query failure.
    async fn find_by_status(&self, status: SagaStatus) -> AppResult<Vec<SagaInstance>>;
}

// ─── SagaOrchestrator trait ──────────────────────────────────────────────────

/// Drives the saga state machine.
///
/// Implementations live in the application layer; the shared-kernel only
/// defines the port so that inner layers can reference it.
#[async_trait]
pub trait SagaOrchestrator: Send + Sync {
    /// Start a new saga execution.
    ///
    /// Creates the [`SagaInstance`], persists it, and emits the first step
    /// command via the outbox.
    ///
    /// # Errors
    /// Returns an error when persistence or outbox insertion fails.
    async fn start(&self, saga_type: &str, payload: Value) -> AppResult<Uuid>;

    /// Handle a step response (success).
    ///
    /// Advances to the next step or marks the saga as completed.
    ///
    /// # Errors
    /// Returns an error on persistence failure.
    async fn on_step_completed(
        &self,
        saga_id: Uuid,
        step_index: usize,
        response: Value,
    ) -> AppResult<()>;

    /// Handle a step failure.
    ///
    /// Begins compensating in reverse from the last completed step.
    ///
    /// # Errors
    /// Returns an error on persistence failure.
    async fn on_step_failed(
        &self,
        saga_id: Uuid,
        step_index: usize,
        error: String,
    ) -> AppResult<()>;

    /// Handle a compensation step response.
    ///
    /// Continues compensating backwards or marks the saga as
    /// `CompensationCompleted`.
    ///
    /// # Errors
    /// Returns an error on persistence failure.
    async fn on_compensation_completed(
        &self,
        saga_id: Uuid,
        step_index: usize,
    ) -> AppResult<()>;
}
