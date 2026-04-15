//! SeaORM-backed [`UnitOfWork`] adapter.

use async_trait::async_trait;
use ddd_application::{UnitOfWork, UnitOfWorkFactory};
use ddd_shared_kernel::{AppError, AppResult};
use sea_orm::{DatabaseConnection, DatabaseTransaction, TransactionTrait};

/// A transactional scope backed by a SeaORM [`DatabaseTransaction`].
pub struct SeaOrmUnitOfWork {
    tx: Option<DatabaseTransaction>,
}

impl SeaOrmUnitOfWork {
    /// Open a new transaction on `db`.
    pub async fn begin(db: &DatabaseConnection) -> AppResult<Self> {
        let tx = db
            .begin()
            .await
            .map_err(|e| AppError::database(e.to_string()))?;
        Ok(Self { tx: Some(tx) })
    }

    /// Borrow the underlying transaction for use by repositories.
    pub fn tx(&self) -> Option<&DatabaseTransaction> {
        self.tx.as_ref()
    }
}

#[async_trait]
impl UnitOfWork for SeaOrmUnitOfWork {
    async fn commit(mut self: Box<Self>) -> AppResult<()> {
        if let Some(tx) = self.tx.take() {
            tx.commit().await.map_err(|e| AppError::database(e.to_string()))?;
        }
        Ok(())
    }

    async fn rollback(mut self: Box<Self>) -> AppResult<()> {
        if let Some(tx) = self.tx.take() {
            tx.rollback().await.map_err(|e| AppError::database(e.to_string()))?;
        }
        Ok(())
    }
}

/// Factory that opens [`SeaOrmUnitOfWork`] instances.
pub struct SeaOrmUnitOfWorkFactory {
    db: DatabaseConnection,
}

impl SeaOrmUnitOfWorkFactory {
    /// Create a new factory.
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UnitOfWorkFactory for SeaOrmUnitOfWorkFactory {
    type Uow = SeaOrmUnitOfWork;

    async fn begin(&self) -> AppResult<Self::Uow> {
        SeaOrmUnitOfWork::begin(&self.db).await
    }
}
