//! sqlx-based migration runner.
//!
//! Uses sqlx directly to apply SQL migration files from a given directory.

use sea_orm::DbErr;
use sqlx::postgres::PgPoolOptions;

/// Apply all migrations at the default path (`./migrations`).
pub async fn run_migrations(database_url: &str) -> Result<(), DbErr> {
    run_migrations_from_path(database_url, "./migrations").await
}

/// Apply migrations from a custom path using sqlx runtime migrator.
pub async fn run_migrations_from_path(database_url: &str, path: &str) -> Result<(), DbErr> {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(database_url)
        .await
        .map_err(|e| DbErr::Custom(format!("migration pool connect: {e}")))?;

    let migrator = sqlx::migrate::Migrator::new(std::path::Path::new(path))
        .await
        .map_err(|e| DbErr::Custom(format!("load migrations: {e}")))?;
    migrator
        .run(&pool)
        .await
        .map_err(|e| DbErr::Custom(format!("apply migrations: {e}")))?;
    Ok(())
}
