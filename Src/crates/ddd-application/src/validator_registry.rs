//! Transport-agnostic registry of [`Validator`]s keyed by the type they
//! validate. Populated at link time through the [`inventory`] crate so REST
//! extractors and gRPC handlers can look up "the validator for `T`" without
//! a hand-maintained wire-up list.
//!
//! # Authoring a validator
//!
//! ```ignore
//! use ddd_application::{Validator, register_validator};
//! use ddd_application::validation::{ValidationResult, ValidationRule};
//!
//! pub struct CreateOrder { pub sku: String, pub qty: u32 }
//!
//! pub struct CreateOrderValidator;
//! impl Validator<CreateOrder> for CreateOrderValidator {
//!     fn validate(&self, v: &CreateOrder) -> ValidationResult {
//!         let mut r = ValidationResult::new();
//!         r.merge(ValidationRule::new(v.sku.as_str(), "sku").not_empty().max_length(64).into());
//!         r.merge(ValidationRule::new(&v.qty, "qty").positive().into());
//!         r
//!     }
//! }
//!
//! register_validator!(CreateOrder, CreateOrderValidator);
//! ```
//!
//! # Building the registry
//!
//! ```ignore
//! let validators = ValidatorRegistry::from_inventory();
//! validators.validate(&create_order)?;   // AppResult<()>
//! ```

use std::any::{Any, TypeId};
use std::sync::Arc;

use ddd_shared_kernel::{AppResult, validation::ValidationResult};
use rustc_hash::FxHashMap;

use crate::validation::Validator;

#[doc(hidden)]
pub use inventory;

// ─── Erased validator ────────────────────────────────────────────────────────

/// Object-safe form of [`Validator<T>`] used for registry storage.
pub trait ErasedValidator: Send + Sync {
    /// Run validation against an `&dyn Any` pointing at a `T`.
    fn validate_erased(&self, value: &dyn Any) -> ValidationResult;
}

struct TypedValidator<T, V: Validator<T>> {
    inner: V,
    _t: std::marker::PhantomData<fn() -> T>,
}

impl<T, V> ErasedValidator for TypedValidator<T, V>
where
    T: 'static,
    V: Validator<T> + 'static,
{
    fn validate_erased(&self, value: &dyn Any) -> ValidationResult {
        match value.downcast_ref::<T>() {
            Some(v) => self.inner.validate(v),
            None => {
                let mut r = ValidationResult::new();
                r.merge(ddd_shared_kernel::validation::ValidationRule::new(&"", "__type")
                    .must(|_| false, "validator type mismatch", "type_mismatch")
                    .into());
                r
            }
        }
    }
}

// ─── Inventory slot ──────────────────────────────────────────────────────────

/// Link-time registration record produced by [`register_validator!`].
pub struct ValidatorRegistration {
    /// Human-readable name of the validated type (for error messages).
    pub name: &'static str,
    /// Installs the validator into the registry.
    pub register: fn(&mut ValidatorRegistry),
}

inventory::collect!(ValidatorRegistration);

// ─── Registry ────────────────────────────────────────────────────────────────

/// Map `TypeId<T>` → registered validator. Immutable after construction.
#[derive(Default, Clone)]
pub struct ValidatorRegistry {
    validators: FxHashMap<TypeId, Arc<dyn ErasedValidator>>,
}

impl ValidatorRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a validator for `T`. Replaces any previous validator for `T`.
    pub fn register<T, V>(&mut self, validator: V) -> &mut Self
    where
        T: 'static,
        V: Validator<T> + 'static,
    {
        let erased: Arc<dyn ErasedValidator> = Arc::new(TypedValidator::<T, V> {
            inner: validator,
            _t: std::marker::PhantomData,
        });
        self.validators.insert(TypeId::of::<T>(), erased);
        self
    }

    /// Collect every validator registered through [`register_validator!`].
    pub fn from_inventory() -> Self {
        let mut reg = Self::default();
        for r in inventory::iter::<ValidatorRegistration> {
            (r.register)(&mut reg);
        }
        reg
    }

    /// Run the validator registered for `T`. Returns `Ok(())` when no
    /// validator is registered — callers decide whether absence is an error.
    #[inline]
    pub fn validate<T: 'static>(&self, value: &T) -> AppResult<()> {
        match self.validators.get(&TypeId::of::<T>()) {
            None => Ok(()),
            Some(v) => v.validate_erased(value).into_app_error(),
        }
    }

    /// Same as [`validate`](Self::validate) but returns the raw
    /// [`ValidationResult`] so callers can inspect individual failures.
    #[inline]
    pub fn validate_raw<T: 'static>(&self, value: &T) -> ValidationResult {
        match self.validators.get(&TypeId::of::<T>()) {
            None => ValidationResult::new(),
            Some(v) => v.validate_erased(value),
        }
    }

    /// `true` when a validator is registered for `T`.
    pub fn has<T: 'static>(&self) -> bool {
        self.validators.contains_key(&TypeId::of::<T>())
    }

    /// Number of registered validators.
    pub fn len(&self) -> usize {
        self.validators.len()
    }

    /// `true` when no validators are registered.
    pub fn is_empty(&self) -> bool {
        self.validators.is_empty()
    }
}

// ─── Registration macro ──────────────────────────────────────────────────────

/// Register a [`Validator`] at link time. The third argument is optional and
/// defaults to `<$validator>::default()`; pass an explicit constructor
/// expression when the validator is not `Default`.
///
/// ```ignore
/// register_validator!(CreateOrder, CreateOrderValidator);
/// register_validator!(UpdateOrder, UpdateOrderValidator::new(cfg));
/// ```
#[macro_export]
macro_rules! register_validator {
    ($target:ty, $validator:ty) => {
        $crate::validator_registry::inventory::submit! {
            $crate::validator_registry::ValidatorRegistration {
                name: ::std::any::type_name::<$target>(),
                register: |reg| {
                    reg.register::<$target, _>(<$validator as ::std::default::Default>::default());
                },
            }
        }
    };
    ($target:ty, $validator:expr) => {
        $crate::validator_registry::inventory::submit! {
            $crate::validator_registry::ValidatorRegistration {
                name: ::std::any::type_name::<$target>(),
                register: |reg| {
                    reg.register::<$target, _>($validator);
                },
            }
        }
    };
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ddd_shared_kernel::validation::ValidationRule;

    struct Dto {
        name: String,
    }

    #[derive(Default)]
    struct DtoValidator;
    impl Validator<Dto> for DtoValidator {
        fn validate(&self, v: &Dto) -> ValidationResult {
            ValidationRule::new(v.name.as_str(), "name").not_empty().into()
        }
    }

    #[test]
    fn registers_and_runs() {
        let mut reg = ValidatorRegistry::new();
        reg.register::<Dto, _>(DtoValidator);

        assert!(reg.validate(&Dto { name: "x".into() }).is_ok());
        let err = reg.validate(&Dto { name: String::new() }).unwrap_err();
        assert!(err.is_validation());
    }

    #[test]
    fn missing_validator_is_ok() {
        let reg = ValidatorRegistry::new();
        assert!(reg.validate(&Dto { name: String::new() }).is_ok());
    }

    #[test]
    fn raw_result_surfaces_fields() {
        let mut reg = ValidatorRegistry::new();
        reg.register::<Dto, _>(DtoValidator);
        let r = reg.validate_raw(&Dto { name: String::new() });
        assert!(!r.is_valid());
        assert_eq!(r.errors()[0].field, "name");
    }
}
