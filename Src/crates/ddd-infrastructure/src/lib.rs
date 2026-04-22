//! # ddd-infrastructure
//!
//! Concrete infrastructure adapters for DDD microservices:
//!
//! - [`db`] — SeaORM database connection, generic repository, unit of work,
//!   and outbox / inbox repositories.
//! - [`messaging`] — NATS publisher / subscriber and JSON event
//!   (de)serialisation helpers.
//! - [`telemetry`] — OpenTelemetry tracing, Prometheus metrics, and
//!   structured JSON logging initialisation.
//!
//! ## Feature flags
//!
//! | Feature | Enables |
//! |---------|---------|
//! | `postgres` (default) | SeaORM-backed repositories in [`db`] |
//! | `nats` (default) | NATS publisher / subscriber in [`messaging`] |
//! | `nats-jetstream` | JetStream durable publisher / pull-subscriber |
//! | `storage` | S3-compatible [`BlobStorage`] adapter in [`storage`] |
//! | `crypto` | PBKDF2 / AES-GCM adapters in [`security`] |
//! | `cache` | Redis adapter in [`cache`] |
//! | `telemetry` (default) | OpenTelemetry + Prometheus init in [`telemetry`] |
//! | `full` | All of the above |

#![warn(missing_docs)]

#[cfg(feature = "cache")]
pub mod cache;
pub mod db;
pub mod messaging;
#[cfg(feature = "crypto")]
pub mod security;
#[cfg(feature = "storage")]
pub mod storage;
pub mod telemetry;

// ─── Crate-root re-exports ───────────────────────────────────────────────────

#[cfg(feature = "cache")]
pub use cache::RedisCache;
pub use db::{
    create_pool, create_pool_from_env, run_migrations, run_migrations_from_path, BaseRepository,
    DbIdempotencyStore, SeaOrmDeadLetterRepository, SeaOrmInboxRepository, SeaOrmOutboxRepository,
    SeaOrmSagaInstanceRepository, SeaOrmUnitOfWork, SeaOrmUnitOfWorkFactory,
};
pub use messaging::{
    deserialize_event, serialize_event, JetStreamPublisher, JetStreamSubscriber,
    JetStreamSubscriberConfig, NatsPublisher, NatsSubscriber,
};
#[cfg(feature = "crypto")]
pub use security::{AesGcmCipher, Pbkdf2Hasher};
#[cfg(feature = "storage")]
pub use storage::{S3BlobStorage, S3Config};
pub use telemetry::{
    init_logging, init_metrics, init_telemetry, init_tracing, metrics_handler, shutdown_telemetry,
    shutdown_tracing, Metrics,
};
