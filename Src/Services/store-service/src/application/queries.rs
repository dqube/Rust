use ddd_application::impl_query;

use crate::application::dtos::{PagedRegisterDto, PagedStoreDto, RegisterDto, StoreDto};
use crate::domain::{
    enums::StoreStatus,
    ids::{RegisterId, StoreId},
};

// ── Store queries ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GetStore {
    pub store_id: StoreId,
}
impl_query!(GetStore, Option<StoreDto>);

#[derive(Debug, Clone)]
pub struct ListStores {
    pub page:        i32,
    pub page_size:   i32,
    pub search_term: Option<String>,
    pub status:      Option<StoreStatus>,
    pub location_id: Option<i32>,
}
impl_query!(ListStores, PagedStoreDto);

#[derive(Debug, Clone)]
pub struct GetStoreLogoUrl {
    pub store_id: StoreId,
}
impl_query!(GetStoreLogoUrl, Option<String>);

// ── Register queries ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GetRegister {
    pub register_id: RegisterId,
}
impl_query!(GetRegister, Option<RegisterDto>);

#[derive(Debug, Clone)]
pub struct ListRegisters {
    pub store_id:  StoreId,
    pub page:      i32,
    pub page_size: i32,
}
impl_query!(ListRegisters, PagedRegisterDto);
