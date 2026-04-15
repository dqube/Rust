//! Entity and value-object marker traits.

use std::fmt;

// ─── Entity ──────────────────────────────────────────────────────────────────

/// Marker trait for domain entities.
///
/// An entity has a meaningful identity independent of its attributes.
/// Equality is based on identity, not attribute values.
pub trait Entity {
    /// The type used to identify this entity.
    type Id: fmt::Display + Clone + PartialEq + Send + Sync;

    /// Return a reference to the entity's identity.
    fn id(&self) -> &Self::Id;
}

// ─── ValueObject ─────────────────────────────────────────────────────────────

/// Marker trait for value objects.
///
/// A value object has no identity; two value objects with the same attributes
/// are considered equal.  Implementors must be `Clone` and `PartialEq`.
pub trait ValueObject: Clone + PartialEq + fmt::Debug {}
