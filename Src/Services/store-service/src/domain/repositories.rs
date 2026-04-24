use async_trait::async_trait;
use ddd_shared_kernel::AppResult;

use crate::domain::{
    entities::{Register, Store},
    enums::StoreStatus,
    ids::{RegisterId, StoreId},
};

#[derive(Debug, Clone)]
pub struct PagedResult<T> {
    pub items:     Vec<T>,
    pub total:     i64,
    pub page:      i32,
    pub page_size: i32,
}

impl<T> PagedResult<T> {
    pub fn new(items: Vec<T>, total: i64, page: i32, page_size: i32) -> Self {
        Self { items, total, page, page_size }
    }
}

#[async_trait]
pub trait StoreRepository: Send + Sync {
    async fn find_by_id(&self, id: StoreId) -> AppResult<Option<Store>>;
    async fn name_exists(&self, name: &str) -> AppResult<bool>;
    async fn get_paged(
        &self,
        page:        i32,
        page_size:   i32,
        search:      Option<&str>,
        status:      Option<StoreStatus>,
        location_id: Option<i32>,
    ) -> AppResult<PagedResult<Store>>;
    async fn save(&self, store: &mut Store) -> AppResult<()>;
}

#[async_trait]
pub trait RegisterRepository: Send + Sync {
    async fn find_by_id(&self, id: RegisterId) -> AppResult<Option<Register>>;
    async fn name_exists_in_store(&self, store_id: StoreId, name: &str) -> AppResult<bool>;
    async fn get_by_store_id(&self, store_id: StoreId) -> AppResult<Vec<Register>>;
    async fn get_paged(
        &self,
        store_id:  StoreId,
        page:      i32,
        page_size: i32,
    ) -> AppResult<PagedResult<Register>>;
    async fn save(&self, register: &mut Register) -> AppResult<()>;
}
