//! SeaORM `DatabaseConnection` construction helpers.

use sea_orm::{Database, DatabaseConnection, DbErr};

/// Create a connection pool from a connection URL.
pub async fn create_pool(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    Database::connect(database_url).await
}

/// Create a connection pool from the `DATABASE_URL` environment variable.
pub async fn create_pool_from_env() -> Result<DatabaseConnection, DbErr> {
    let url = std::env::var("DATABASE_URL")
        .map_err(|_| DbErr::Custom("DATABASE_URL not set".to_owned()))?;
    create_pool(&url).await
}
