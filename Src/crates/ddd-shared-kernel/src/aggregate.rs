//! Aggregate root trait and helper macros.
//!
//! An *aggregate root* is the entry point for a consistency boundary in DDD.
//! It owns a collection of uncommitted domain events that are dispatched after
//! persistence.

use chrono::{DateTime, Utc};

use crate::domain_event::DomainEvent;

// ─── AggregateRoot ───────────────────────────────────────────────────────────

/// Behaviour every aggregate root must expose.
pub trait AggregateRoot {
    /// The strongly-typed identifier for this aggregate.
    type Id: std::fmt::Display + Clone + Send + Sync;

    /// The aggregate's identity.
    fn id(&self) -> &Self::Id;

    /// Optimistic-concurrency version counter.
    fn version(&self) -> u64;

    /// Update the version counter (called by the repository after save).
    fn set_version(&mut self, version: u64);

    /// When the aggregate was last modified.
    fn updated_at(&self) -> DateTime<Utc>;

    /// Borrow the pending domain events without removing them.
    fn domain_events(&self) -> &[Box<dyn DomainEvent>];

    /// Drain the pending domain events.  The aggregate's internal buffer is
    /// cleared and the events are returned to the caller for dispatch.
    fn take_domain_events(&mut self) -> Vec<Box<dyn DomainEvent>>;

    /// Discard all pending domain events without dispatching them.
    fn clear_domain_events(&mut self);
}

// ─── impl_aggregate_root! ────────────────────────────────────────────────────

/// Implement [`AggregateRoot`] for a struct that follows the standard layout.
///
/// The struct must have the following fields:
///
/// | Field | Type |
/// |-------|------|
/// | `id` | `$id_type` |
/// | `version` | `u64` |
/// | `updated_at` | `chrono::DateTime<chrono::Utc>` |
/// | `domain_events` | `Vec<Box<dyn DomainEvent>>` |
///
/// # Example
/// ```
/// use ddd_shared_kernel::{declare_id, impl_aggregate_root};
/// use ddd_shared_kernel::domain_event::DomainEvent;
/// use chrono::{DateTime, Utc};
///
/// declare_id!(UserId);
///
/// struct User {
///     id: UserId,
///     version: u64,
///     updated_at: DateTime<Utc>,
///     domain_events: Vec<Box<dyn DomainEvent>>,
///     name: String,
/// }
///
/// impl_aggregate_root!(User, UserId);
/// ```
#[macro_export]
macro_rules! impl_aggregate_root {
    ($struct:ty, $id_type:ty) => {
        impl $crate::aggregate::AggregateRoot for $struct {
            type Id = $id_type;

            fn id(&self) -> &Self::Id {
                &self.id
            }

            fn version(&self) -> u64 {
                self.version
            }

            fn set_version(&mut self, version: u64) {
                self.version = version;
            }

            fn updated_at(&self) -> ::chrono::DateTime<::chrono::Utc> {
                self.updated_at
            }

            fn domain_events(&self) -> &[Box<dyn $crate::domain_event::DomainEvent>] {
                &self.domain_events
            }

            fn take_domain_events(&mut self) -> Vec<Box<dyn $crate::domain_event::DomainEvent>> {
                ::std::mem::take(&mut self.domain_events)
            }

            fn clear_domain_events(&mut self) {
                self.domain_events.clear();
            }
        }
    };
}

// ─── record_event! ───────────────────────────────────────────────────────────

/// Push a domain event onto the aggregate's pending-event buffer.
///
/// # Example
/// ```
/// # use ddd_shared_kernel::{declare_id, impl_aggregate_root, record_event};
/// # use ddd_shared_kernel::domain_event::DomainEvent;
/// # use chrono::{DateTime, Utc};
/// # use std::any::Any;
/// #
/// # declare_id!(UserId);
/// #
/// # #[derive(Debug)]
/// # struct UserCreated { occurred_at: DateTime<Utc> }
/// # impl DomainEvent for UserCreated {
/// #     fn event_name(&self) -> &'static str { "user.created" }
/// #     fn occurred_at(&self) -> DateTime<Utc> { self.occurred_at }
/// #     fn as_any(&self) -> &dyn Any { self }
/// # }
/// #
/// # struct User {
/// #     id: UserId, version: u64, updated_at: DateTime<Utc>,
/// #     domain_events: Vec<Box<dyn DomainEvent>>,
/// # }
/// # impl_aggregate_root!(User, UserId);
/// #
/// # let mut user = User {
/// #     id: UserId::new(), version: 0, updated_at: Utc::now(),
/// #     domain_events: vec![],
/// # };
/// record_event!(user, UserCreated { occurred_at: chrono::Utc::now() });
/// ```
#[macro_export]
macro_rules! record_event {
    ($aggregate:expr, $event:expr) => {
        $aggregate.domain_events.push(Box::new($event))
    };
}
