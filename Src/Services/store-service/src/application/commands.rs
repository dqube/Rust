use bytes::Bytes;
use ddd_application::impl_command;
use rust_decimal::Decimal;

use crate::application::dtos::{RegisterDto, StoreDto};
use crate::domain::{
    entities::StoreSchedule,
    enums::StoreStatus,
    ids::{RegisterId, StoreId},
};

// ── Store commands ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CreateStore {
    pub name:                String,
    pub location_id:         i32,
    pub address_street:      String,
    pub address_city:        String,
    pub address_postal_code: String,
    pub address_country:     String,
    pub phone:               String,
    pub geo_latitude:        f64,
    pub geo_longitude:       f64,
    pub schedules:           Vec<StoreSchedule>,
    pub services:            Vec<String>,
}
impl_command!(CreateStore, StoreDto);

#[derive(Debug, Clone)]
pub struct UpdateStoreInformation {
    pub store_id:            StoreId,
    pub name:                String,
    pub address_street:      String,
    pub address_city:        String,
    pub address_postal_code: String,
    pub address_country:     String,
    pub phone:               String,
    pub geo_latitude:        f64,
    pub geo_longitude:       f64,
    pub schedules:           Vec<StoreSchedule>,
    pub services:            Vec<String>,
}
impl_command!(UpdateStoreInformation, StoreDto);

#[derive(Debug, Clone)]
pub struct ChangeStoreStatus {
    pub store_id: StoreId,
    pub status:   StoreStatus,
}
impl_command!(ChangeStoreStatus, ());

#[derive(Debug, Clone)]
pub struct UploadStoreLogo {
    pub store_id:     StoreId,
    pub file_content: Bytes,
    pub file_name:    String,
    pub content_type: String,
}
impl_command!(UploadStoreLogo, String);

#[derive(Debug, Clone)]
pub struct DeleteStoreLogo {
    pub store_id: StoreId,
}
impl_command!(DeleteStoreLogo, ());

// ── Register commands ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CreateRegister {
    pub store_id: StoreId,
    pub name:     String,
}
impl_command!(CreateRegister, RegisterDto);

#[derive(Debug, Clone)]
pub struct OpenRegister {
    pub register_id:   RegisterId,
    pub starting_cash: Decimal,
}
impl_command!(OpenRegister, ());

#[derive(Debug, Clone)]
pub struct CloseRegister {
    pub register_id: RegisterId,
    pub ending_cash: Decimal,
}
impl_command!(CloseRegister, ());

#[derive(Debug, Clone)]
pub struct AddCashToRegister {
    pub register_id: RegisterId,
    pub amount:      Decimal,
    pub note:        String,
}
impl_command!(AddCashToRegister, ());

#[derive(Debug, Clone)]
pub struct RemoveCashFromRegister {
    pub register_id: RegisterId,
    pub amount:      Decimal,
    pub note:        String,
}
impl_command!(RemoveCashFromRegister, ());

#[derive(Debug, Clone)]
pub struct UpdateRegisterName {
    pub register_id: RegisterId,
    pub new_name:    String,
}
impl_command!(UpdateRegisterName, RegisterDto);
