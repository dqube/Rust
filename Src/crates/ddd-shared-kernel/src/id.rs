//! Typed identifiers for domain entities.
//!
//! [`TypedId<T>`] wraps a [`uuid::Uuid`] and carries a phantom type parameter so
//! that IDs for different domain objects are distinct at the type level.
//!
//! # Example
//! ```
//! use ddd_shared_kernel::declare_id;
//!
//! declare_id!(UserId);
//!
//! let id = UserId::new();
//! println!("{}", id); // prints the UUID string
//! ```

use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// A strongly-typed wrapper around a [`Uuid`].
///
/// The phantom type parameter `T` prevents accidental mixing of IDs for
/// different entity types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypedId<T>(Uuid, PhantomData<T>);

impl<T> TypedId<T> {
    /// Generate a new random v7 UUID-backed identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7(), PhantomData)
    }

    /// Wrap an existing [`Uuid`].
    #[must_use]
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid, PhantomData)
    }

    /// Return the inner [`Uuid`].
    #[must_use]
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Parse a [`TypedId`] from a UUID string.
    ///
    /// # Errors
    /// Returns a [`uuid::Error`] when the string is not a valid UUID.
    pub fn parse_str(s: &str) -> Result<Self, uuid::Error> {
        Uuid::parse_str(s).map(|u| Self(u, PhantomData))
    }
}

impl<T> Default for TypedId<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> fmt::Display for TypedId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T> FromStr for TypedId<T> {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s)
    }
}

impl<T> Serialize for TypedId<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for TypedId<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Uuid::deserialize(deserializer).map(|u| Self(u, PhantomData))
    }
}

/// Declare a newtype identifier backed by [`TypedId`].
///
/// This macro generates a zero-cost newtype that wraps `TypedId<Self>` and
/// re-exports all its constructors and trait implementations.
///
/// # Example
/// ```
/// use ddd_shared_kernel::declare_id;
///
/// declare_id!(OrderId);
///
/// let id = OrderId::new();
/// let _ = id.as_uuid();
/// let _ = id.to_string();
/// ```
#[macro_export]
macro_rules! declare_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
            ::serde::Serialize, ::serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name($crate::TypedId<$name>);

        impl $name {
            /// Generate a new random identifier.
            #[must_use]
            pub fn new() -> Self {
                Self($crate::TypedId::new())
            }

            /// Wrap an existing [`uuid::Uuid`].
            #[must_use]
            #[allow(dead_code)]
            pub fn from_uuid(uuid: ::uuid::Uuid) -> Self {
                Self($crate::TypedId::from_uuid(uuid))
            }

            /// Return the inner [`uuid::Uuid`].
            #[must_use]
            #[allow(dead_code)]
            pub fn as_uuid(&self) -> ::uuid::Uuid {
                self.0.as_uuid()
            }

            /// Parse from a UUID string.
            ///
            /// # Errors
            /// Returns a [`uuid::Error`] if the string is not a valid UUID.
            pub fn parse_str(s: &str) -> Result<Self, ::uuid::Error> {
                $crate::TypedId::parse_str(s).map(Self)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = ::uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::parse_str(s)
            }
        }
    };
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct User;
    #[derive(Debug, PartialEq)]
    struct Order;

    #[test]
    fn new_ids_are_unique() {
        let a = TypedId::<User>::new();
        let b = TypedId::<User>::new();
        assert_ne!(a, b);
    }

    #[test]
    fn round_trip_display_parse() {
        let id = TypedId::<User>::new();
        let s = id.to_string();
        let parsed: TypedId<User> = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn typed_ids_are_type_safe() {
        let user_id = TypedId::<User>::new();
        let order_id = TypedId::<Order>::new();
        // The following would not compile if uncommented:
        // assert_eq!(user_id, order_id);
        let _ = (user_id, order_id);
    }

    #[test]
    fn serde_round_trip() {
        let id = TypedId::<User>::new();
        let json = serde_json::to_string(&id).unwrap();
        let back: TypedId<User> = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn declare_id_macro() {
        declare_id!(ProductId);

        let a = ProductId::new();
        let b = ProductId::new();
        assert_ne!(a, b);

        let s = a.to_string();
        let parsed: ProductId = s.parse().unwrap();
        assert_eq!(a, parsed);
    }
}
