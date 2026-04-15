//! Unit-of-work abstraction — transactional boundary for application services.

use async_trait::async_trait;
use ddd_shared_kernel::AppResult;

/// A transactional scope. Implementations typically wrap a database
/// transaction; the work is committed or rolled back exactly once.
#[async_trait]
pub trait UnitOfWork: Send + Sync {
    /// Commit the transaction, consuming the UoW.
    async fn commit(self: Box<Self>) -> AppResult<()>;

    /// Roll back the transaction, consuming the UoW.
    async fn rollback(self: Box<Self>) -> AppResult<()>;
}

/// Factory that opens a new [`UnitOfWork`] for each invocation.
#[async_trait]
pub trait UnitOfWorkFactory: Send + Sync {
    /// The concrete UoW type produced.
    type Uow: UnitOfWork;

    /// Begin a new transactional scope.
    async fn begin(&self) -> AppResult<Self::Uow>;
}

/// Run `body` inside a transactional scope opened by `$factory`.
///
/// On `Ok` the UoW is committed; on `Err` (or panic-unwind from the closure)
/// it is rolled back and the error propagated.
///
/// # Example
/// ```ignore
/// use ddd_application::transactional;
/// # async fn demo<F: ddd_application::UnitOfWorkFactory>(factory: &F) -> ddd_shared_kernel::AppResult<()> {
/// let _ = transactional!(factory, |_uow| async move {
///     Ok::<(), ddd_shared_kernel::AppError>(())
/// }).await?;
/// # Ok(()) }
/// ```
#[macro_export]
macro_rules! transactional {
    ($factory:expr, $body:expr) => {{
        async {
            let uow = $crate::unit_of_work::UnitOfWorkFactory::begin($factory).await?;
            let mut uow = ::std::boxed::Box::new(uow);
            let body = $body;
            // Temporarily move uow into the closure via raw pointer-free split:
            // pass by &mut to the closure, then commit/rollback based on result.
            let res = body(&mut *uow).await;
            match res {
                ::std::result::Result::Ok(v) => {
                    $crate::unit_of_work::UnitOfWork::commit(uow).await?;
                    ::std::result::Result::Ok(v)
                }
                ::std::result::Result::Err(e) => {
                    let _ = $crate::unit_of_work::UnitOfWork::rollback(uow).await;
                    ::std::result::Result::Err(e)
                }
            }
        }
    }};
}
