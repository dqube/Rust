use std::sync::Arc;

use sea_orm::DatabaseConnection;

pub async fn run_seeder(_db: &Arc<DatabaseConnection>) {}