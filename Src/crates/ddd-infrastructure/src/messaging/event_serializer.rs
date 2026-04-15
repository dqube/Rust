//! JSON (de)serialisation helpers for integration events.

use ddd_shared_kernel::{AppError, AppResult};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

/// Serialise `event` to a [`serde_json::Value`].
pub fn serialize_event<E: Serialize>(event: &E) -> AppResult<Value> {
    serde_json::to_value(event).map_err(|e| AppError::serialization(e.to_string()))
}

/// Deserialise `value` into a `T`.
pub fn deserialize_event<T: DeserializeOwned>(value: &Value) -> AppResult<T> {
    serde_json::from_value(value.clone()).map_err(|e| AppError::serialization(e.to_string()))
}
