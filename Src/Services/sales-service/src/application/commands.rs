use bytes::Bytes;
use ddd_application::impl_command;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::application::dtos::{ReturnDto, SaleDto};
use crate::domain::enums::{ReturnReason, SalesChannel};
use crate::domain::ids::{ReturnId, SaleDetailId, SaleId};

// ── Address (used in SetSaleAddresses) ────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Address {
    pub street:   String,
    pub city:     String,
    pub state:    String,
    pub zip_code: String,
    pub country:  String,
}

impl From<Address> for crate::domain::entities::Address {
    fn from(a: Address) -> Self {
        crate::domain::entities::Address {
            street:   a.street,
            city:     a.city,
            state:    a.state,
            zip_code: a.zip_code,
            country:  a.country,
        }
    }
}

// ── Sale commands ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CreateSale {
    pub store_id:       i32,
    pub employee_id:    Uuid,
    pub register_id:    i32,
    pub receipt_number: String,
    pub customer_id:    Option<Uuid>,
    pub channel:        SalesChannel,
}
impl_command!(CreateSale, SaleDto);

#[derive(Debug, Clone)]
pub struct AddSaleDetail {
    pub sale_id:     SaleId,
    pub product_id:  Uuid,
    pub variant_id:  Option<Uuid>,
    pub quantity:    i32,
    pub unit_price:  Decimal,
    pub tax_applied: Decimal,
}
impl_command!(AddSaleDetail, ());

#[derive(Debug, Clone)]
pub struct UpdateSaleDetail {
    pub sale_id:        SaleId,
    pub sale_detail_id: SaleDetailId,
    pub quantity:       i32,
    pub unit_price:     Decimal,
    pub tax_applied:    Decimal,
}
impl_command!(UpdateSaleDetail, ());

#[derive(Debug, Clone)]
pub struct RemoveSaleDetail {
    pub sale_id:        SaleId,
    pub sale_detail_id: SaleDetailId,
}
impl_command!(RemoveSaleDetail, ());

#[derive(Debug, Clone)]
pub struct ApplyDiscount {
    pub sale_id:         SaleId,
    pub sale_detail_id:  Option<SaleDetailId>,
    pub campaign_id:     Uuid,
    pub rule_id:         Uuid,
    pub discount_amount: Decimal,
}
impl_command!(ApplyDiscount, ());

#[derive(Debug, Clone)]
pub struct CompleteSale {
    pub sale_id: SaleId,
}
impl_command!(CompleteSale, ());

#[derive(Debug, Clone)]
pub struct CancelSale {
    pub sale_id: SaleId,
    pub reason:  String,
}
impl_command!(CancelSale, ());

#[derive(Debug, Clone)]
pub struct UpdateSaleStatus {
    pub sale_id: SaleId,
    pub status:  String,
}
impl_command!(UpdateSaleStatus, ());

#[derive(Debug, Clone)]
pub struct SetSaleAddresses {
    pub sale_id:          SaleId,
    pub shipping_address: Address,
    pub billing_address:  Address,
}
impl_command!(SetSaleAddresses, ());

#[derive(Debug, Clone)]
pub struct SetPaymentTransaction {
    pub sale_id:        SaleId,
    pub transaction_id: String,
}
impl_command!(SetPaymentTransaction, ());

#[derive(Debug, Clone)]
pub struct UploadSaleReceipt {
    pub sale_id:      SaleId,
    pub file_content: Bytes,
    pub file_name:    String,
    pub content_type: String,
}
impl_command!(UploadSaleReceipt, String);

#[derive(Debug, Clone)]
pub struct PlaceOrder {
    pub customer_id: Uuid,
    pub store_id:    i32,
    pub currency:    String,
    pub items:       Vec<(Uuid, i32, Decimal)>,
}
impl_command!(PlaceOrder, SaleDto);

// ── Return commands ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CreateReturn {
    pub sale_id:     SaleId,
    pub employee_id: Uuid,
    pub customer_id: Option<Uuid>,
}
impl_command!(CreateReturn, ReturnDto);

#[derive(Debug, Clone)]
pub struct AddReturnDetail {
    pub return_id:  ReturnId,
    pub product_id: Uuid,
    pub quantity:   i32,
    pub reason:     ReturnReason,
    pub restock:    bool,
}
impl_command!(AddReturnDetail, ());

#[derive(Debug, Clone)]
pub struct ProcessReturn {
    pub return_id:    ReturnId,
    pub total_refund: Decimal,
}
impl_command!(ProcessReturn, ());



