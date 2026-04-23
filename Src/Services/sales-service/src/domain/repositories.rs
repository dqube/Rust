use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ddd_shared_kernel::AppError;
use uuid::Uuid;

use crate::domain::entities::{OrderSaga, Return, Sale};
use crate::domain::ids::{ReturnId, SaleId};

#[async_trait]
pub trait SaleRepository: Send + Sync {
    async fn find_by_id(&self, id: SaleId) -> Result<Option<Sale>, AppError>;
    async fn find_with_details(&self, id: SaleId) -> Result<Option<Sale>, AppError>;
    async fn find_by_receipt(&self, receipt: &str) -> Result<Option<Sale>, AppError>;
    async fn receipt_exists(&self, receipt: &str) -> Result<bool, AppError>;
    async fn get_all(&self, page: i32, page_size: i32, status: Option<String>) -> Result<(Vec<Sale>, u64), AppError>;
    async fn get_by_store(&self, store_id: i32, from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> Result<Vec<Sale>, AppError>;
    async fn get_by_employee(&self, employee_id: Uuid, from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> Result<Vec<Sale>, AppError>;
    async fn get_by_customer(&self, customer_id: Uuid) -> Result<Vec<Sale>, AppError>;
    async fn save(&self, sale: &mut Sale) -> Result<(), AppError>;
}

#[async_trait]
pub trait ReturnRepository: Send + Sync {
    async fn find_by_id(&self, id: ReturnId) -> Result<Option<Return>, AppError>;
    async fn find_with_details(&self, id: ReturnId) -> Result<Option<Return>, AppError>;
    async fn get_by_sale(&self, sale_id: SaleId) -> Result<Vec<Return>, AppError>;
    async fn get_by_employee(&self, employee_id: Uuid, from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> Result<Vec<Return>, AppError>;
    async fn get_by_customer(&self, customer_id: Uuid) -> Result<Vec<Return>, AppError>;
    async fn save(&self, ret: &mut Return) -> Result<(), AppError>;
}

#[async_trait]
pub trait OrderSagaRepository: Send + Sync {
    async fn find_by_order_id(&self, order_id: SaleId) -> Result<Option<OrderSaga>, AppError>;
    async fn save(&self, saga: &mut OrderSaga) -> Result<(), AppError>;
}
