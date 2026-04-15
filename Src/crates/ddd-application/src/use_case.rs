//! Use-case trait and a [`ValidatedUseCase`] decorator.

use async_trait::async_trait;
use ddd_shared_kernel::AppResult;

use crate::validation::Validator;

/// A single application-layer use case.
#[async_trait]
pub trait UseCase<Input>: Send + Sync
where
    Input: Send + 'static,
{
    /// The value returned on success.
    type Output: Send + Sync;

    /// Execute the use case.
    async fn execute(&self, input: Input) -> AppResult<Self::Output>;
}

/// Wraps a [`UseCase`] so its input is validated before `execute` runs.
pub struct ValidatedUseCase<U, V, I> {
    inner: U,
    validator: V,
    _i: std::marker::PhantomData<fn(I)>,
}

impl<U, V, I> ValidatedUseCase<U, V, I> {
    /// Wrap `inner` with `validator`.
    pub fn new(inner: U, validator: V) -> Self {
        Self { inner, validator, _i: std::marker::PhantomData }
    }
}

#[async_trait]
impl<U, V, I> UseCase<I> for ValidatedUseCase<U, V, I>
where
    I: Send + Sync + 'static,
    U: UseCase<I>,
    V: Validator<I>,
{
    type Output = U::Output;

    async fn execute(&self, input: I) -> AppResult<Self::Output> {
        self.validator.validate(&input).into_app_error()?;
        self.inner.execute(input).await
    }
}

/// Implement [`UseCase`] for a struct by delegating to a closure.
///
/// The struct must be defined by the user; this macro only writes the impl.
///
/// # Example
/// ```ignore
/// use ddd_application::impl_use_case;
/// pub struct SayHello;
/// impl_use_case!(SayHello, String, String, |_self, name| async move {
///     Ok(format!("hello, {name}"))
/// });
/// ```
#[macro_export]
macro_rules! impl_use_case {
    ($ty:ty, $input:ty, $output:ty, |$self_ident:ident, $input_ident:ident| $body:expr) => {
        #[::async_trait::async_trait]
        impl $crate::use_case::UseCase<$input> for $ty {
            type Output = $output;
            async fn execute(
                &self,
                $input_ident: $input,
            ) -> ::ddd_shared_kernel::AppResult<$output> {
                let $self_ident = self;
                $body.await
            }
        }
    };
}
