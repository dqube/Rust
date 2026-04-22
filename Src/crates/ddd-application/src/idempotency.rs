//! Idempotency guard for command handlers.
//!
//! [`IdempotentCommandHandler`] wraps an inner handler, ensures each
//! `idempotency_key` is processed at most once, and returns the cached
//! response on duplicate requests.
//!
//! # Usage
//!
//! ```ignore
//! use ddd_application::idempotency::{IdempotentCommand, IdempotentCommandHandler};
//!
//! struct CreateOrder { pub sku: String, pub idempotency_key: String }
//! impl Command for CreateOrder { type Response = uuid::Uuid; }
//! impl IdempotentCommand for CreateOrder {
//!     fn idempotency_key(&self) -> &str { &self.idempotency_key }
//! }
//!
//! let handler = IdempotentCommandHandler::new(inner_handler, store, ttl);
//! ```

use std::sync::Arc;
use std::time::Duration;
use std::marker::PhantomData;

use async_trait::async_trait;
use ddd_shared_kernel::idempotency::IdempotencyStore;
use ddd_shared_kernel::{AppError, AppResult};

use crate::cqrs::{Command, CommandHandler};

// ─── IdempotentCommand ──────────────────────────────────────────────────────

/// Marker trait for commands that carry an idempotency key.
///
/// Implement this on your command struct to opt into idempotency protection.
pub trait IdempotentCommand: Command {
    /// The client-supplied idempotency key for this request.
    fn idempotency_key(&self) -> &str;
}

// ─── IdempotentCommandHandler ───────────────────────────────────────────────

/// Decorates a [`CommandHandler`] with idempotency protection.
///
/// On the first request with a given key, the inner handler runs and its
/// serialised response is cached.  Subsequent requests with the same key
/// return the cached response directly.
///
/// If the inner handler fails, the key is released so the client can retry.
pub struct IdempotentCommandHandler<
    C: Command,
    H: CommandHandler<C> + ?Sized = dyn CommandHandler<C>,
    S: IdempotencyStore + ?Sized = dyn IdempotencyStore,
> {
    inner: Arc<H>,
    store: Arc<S>,
    ttl: Duration,
    _command: PhantomData<fn() -> C>,
}

impl<C, H, S> IdempotentCommandHandler<C, H, S>
where
    C: Command,
    H: CommandHandler<C> + ?Sized,
    S: IdempotencyStore + ?Sized,
{
    /// Create a new idempotent handler wrapping `inner`.
    pub fn new(inner: Arc<H>, store: Arc<S>, ttl: Duration) -> Self {
        Self {
            inner,
            store,
            ttl,
            _command: PhantomData,
        }
    }
}

#[async_trait]
impl<C, H, S> CommandHandler<C> for IdempotentCommandHandler<C, H, S>
where
    C: IdempotentCommand + Send + 'static,
    H: CommandHandler<C> + ?Sized,
    S: IdempotencyStore + ?Sized,
    C::Response: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
{
    async fn handle(&self, command: C) -> AppResult<C::Response> {
        let key = command.idempotency_key().to_owned();

        // 1. Try to acquire the key.
        let acquired = self.store.try_acquire(&key, self.ttl).await?;

        if !acquired {
            // The key already exists — check for a cached response.
            if let Some(record) = self.store.get_response(&key).await? {
                let cached: C::Response =
                    serde_json::from_value(record.response).map_err(|e| {
                        AppError::internal(format!(
                            "failed to deserialise cached idempotency response: {e}"
                        ))
                    })?;
                return Ok(cached);
            }
            // Key exists but no response stored yet — another request is
            // in flight. Reject as a conflict.
            return Err(AppError::conflict(
                "duplicate request: this idempotency key is already being processed",
            ));
        }

        // 2. Execute the inner handler.
        match self.inner.handle(command).await {
            Ok(response) => {
                // 3. Cache the response.
                let serialised = serde_json::to_value(&response).map_err(|e| {
                    AppError::internal(format!("failed to serialise idempotency response: {e}"))
                })?;
                // Best-effort: if caching fails, the command still succeeded.
                if let Err(_e) = self.store.store_response(&key, &serialised).await {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        idempotency_key = %key,
                        error = %_e,
                        "Failed to cache idempotency response"
                    );
                }
                Ok(response)
            }
            Err(err) => {
                // 4. Release the key so the client can retry.
                if let Err(_e) = self.store.release(&key).await {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        idempotency_key = %key,
                        error = %_e,
                        "Failed to release idempotency key after error"
                    );
                }
                Err(err)
            }
        }
    }
}
