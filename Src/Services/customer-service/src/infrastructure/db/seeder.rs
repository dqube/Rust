//! Seeder for the customer schema. Currently a no-op — membership numbers
//! and customers are created through the CreateCustomer / EnsureCustomerExists
//! commands rather than pre-seeded.

use std::sync::Arc;

use sea_orm::DatabaseConnection;

pub async fn run_seeder(_db: &Arc<DatabaseConnection>) {
    tracing::info!("Customer seeder completed (no built-in data).");
}
