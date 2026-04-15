//! Value object macro helpers.
//!
//! The [`ValueObject`](crate::ValueObject) trait itself lives in
//! [`crate::entity`] (and is re-exported at the crate root). This module only
//! provides the [`impl_value_object!`] macro for quick blanket impls.

/// Implement [`ValueObject`](crate::ValueObject) for a type that already
/// satisfies the required bounds (`Clone + PartialEq + Debug`).
///
/// # Example
/// ```
/// use ddd_shared_kernel::impl_value_object;
///
/// #[derive(Debug, Clone, PartialEq)]
/// struct Money {
///     amount: u64,
///     currency: String,
/// }
///
/// impl_value_object!(Money);
/// ```
#[macro_export]
macro_rules! impl_value_object {
    ($t:ty) => {
        impl $crate::ValueObject for $t {}
    };
}
