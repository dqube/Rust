//! Database adapters (SeaORM + sqlx migrations).

pub mod base_repository;
pub mod connection;
pub mod dead_letter_repository;
pub mod idempotency_store;
pub mod inbox_repository;
pub mod migration_runner;
pub mod models;
pub mod outbox_repository;
pub mod saga_repository;
pub mod unit_of_work;

pub use base_repository::BaseRepository;
pub use connection::{create_pool, create_pool_from_env};
pub use dead_letter_repository::SeaOrmDeadLetterRepository;
pub use idempotency_store::DbIdempotencyStore;
pub use inbox_repository::SeaOrmInboxRepository;
pub use migration_runner::{run_migrations, run_migrations_from_path};
pub use outbox_repository::SeaOrmOutboxRepository;
pub use saga_repository::SeaOrmSagaInstanceRepository;
pub use unit_of_work::{SeaOrmUnitOfWork, SeaOrmUnitOfWorkFactory};
