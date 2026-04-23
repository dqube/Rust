use chrono::{DateTime, Utc};
use ddd_application::impl_query;
use uuid::Uuid;

use crate::application::dtos::{ReturnDto, SaleDto};
use crate::domain::ids::{ReturnId, SaleId};

#[derive(Debug, Clone)]
pub struct GetSaleById { pub sale_id: SaleId }
impl_query!(GetSaleById, Option<SaleDto>);

#[derive(Debug, Clone)]
pub struct GetSaleByReceipt { pub receipt_number: String }
impl_query!(GetSaleByReceipt, Option<SaleDto>);

#[derive(Debug, Clone)]
pub struct GetSales {
    pub page:      i32,
    pub page_size: i32,
    pub status:    Option<String>,
}
impl_query!(GetSales, (Vec<SaleDto>, u64));

#[derive(Debug, Clone)]
pub struct GetSalesByStore {
    pub store_id:  i32,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date:   Option<DateTime<Utc>>,
}
impl_query!(GetSalesByStore, Vec<SaleDto>);

#[derive(Debug, Clone)]
pub struct GetSalesByEmployee {
    pub employee_id: Uuid,
    pub from_date:   Option<DateTime<Utc>>,
    pub to_date:     Option<DateTime<Utc>>,
}
impl_query!(GetSalesByEmployee, Vec<SaleDto>);

#[derive(Debug, Clone)]
pub struct GetSalesByCustomer { pub customer_id: Uuid }
impl_query!(GetSalesByCustomer, Vec<SaleDto>);

#[derive(Debug, Clone)]
pub struct GetSaleReceiptUrl { pub sale_id: SaleId }
impl_query!(GetSaleReceiptUrl, Option<String>);

#[derive(Debug, Clone)]
pub struct GetReturnById { pub return_id: ReturnId }
impl_query!(GetReturnById, Option<ReturnDto>);

#[derive(Debug, Clone)]
pub struct GetReturnsBySale { pub sale_id: SaleId }
impl_query!(GetReturnsBySale, Vec<ReturnDto>);

#[derive(Debug, Clone)]
pub struct GetReturnsByEmployee {
    pub employee_id: Uuid,
    pub from_date:   Option<DateTime<Utc>>,
    pub to_date:     Option<DateTime<Utc>>,
}
impl_query!(GetReturnsByEmployee, Vec<ReturnDto>);

#[derive(Debug, Clone)]
pub struct GetReturnsByCustomer { pub customer_id: Uuid }
impl_query!(GetReturnsByCustomer, Vec<ReturnDto>);

