use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::entities::{AppliedDiscount, Return, ReturnDetail, Sale, SaleDetail};

#[derive(Debug, Clone)]
pub struct SaleDetailDto {
    pub id:               Uuid,
    pub sale_id:          Uuid,
    pub product_id:       Uuid,
    pub variant_id:       Option<Uuid>,
    pub quantity:         i32,
    pub unit_price:       Decimal,
    pub applied_discount: Decimal,
    pub tax_applied:      Decimal,
    pub line_total:       Decimal,
    pub created_at:       DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AppliedDiscountDto {
    pub id:              Uuid,
    pub sale_id:         Uuid,
    pub sale_detail_id:  Option<Uuid>,
    pub campaign_id:     Uuid,
    pub rule_id:         Uuid,
    pub discount_amount: Decimal,
    pub created_at:      DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SaleDto {
    pub id:                     Uuid,
    pub store_id:               i32,
    pub employee_id:            Uuid,
    pub customer_id:            Option<Uuid>,
    pub register_id:            i32,
    pub receipt_number:         String,
    pub transaction_time:       DateTime<Utc>,
    pub sub_total:              Decimal,
    pub discount_total:         Decimal,
    pub tax_amount:             Decimal,
    pub total_amount:           Decimal,
    pub channel:                String,
    pub status:                 String,
    pub payment_transaction_id: Option<String>,
    pub receipt_object_name:    Option<String>,
    pub created_at:             DateTime<Utc>,
    pub sale_details:           Vec<SaleDetailDto>,
    pub applied_discounts:      Vec<AppliedDiscountDto>,
}

pub fn map_sale(sale: &Sale) -> SaleDto {
    SaleDto {
        id:                     sale.id.as_uuid(),
        store_id:               sale.store_id,
        employee_id:            sale.employee_id,
        customer_id:            sale.customer_id,
        register_id:            sale.register_id,
        receipt_number:         sale.receipt_number.clone(),
        transaction_time:       sale.transaction_time,
        sub_total:              sale.sub_total,
        discount_total:         sale.discount_total,
        tax_amount:             sale.tax_amount,
        total_amount:           sale.total_amount,
        channel:                sale.channel.as_str().to_string(),
        status:                 sale.status.as_str().to_string(),
        payment_transaction_id: sale.payment_transaction_id.clone(),
        receipt_object_name:    sale.receipt_object_name.clone(),
        created_at:             sale.created_at,
        sale_details:           sale.sale_details.iter().map(map_sale_detail).collect(),
        applied_discounts:      sale.applied_discounts.iter().map(map_applied_discount).collect(),
    }
}

pub fn map_sale_detail(d: &SaleDetail) -> SaleDetailDto {
    SaleDetailDto {
        id:               d.id.as_uuid(),
        sale_id:          d.sale_id.as_uuid(),
        product_id:       d.product_id,
        variant_id:       d.variant_id,
        quantity:         d.quantity,
        unit_price:       d.unit_price,
        applied_discount: d.applied_discount,
        tax_applied:      d.tax_applied,
        line_total:       d.line_total,
        created_at:       d.created_at,
    }
}

pub fn map_applied_discount(d: &AppliedDiscount) -> AppliedDiscountDto {
    AppliedDiscountDto {
        id:              d.id.as_uuid(),
        sale_id:         d.sale_id.as_uuid(),
        sale_detail_id:  d.sale_detail_id.map(|i| i.as_uuid()),
        campaign_id:     d.campaign_id,
        rule_id:         d.rule_id,
        discount_amount: d.discount_amount,
        created_at:      d.created_at,
    }
}

#[derive(Debug, Clone)]
pub struct ReturnDetailDto {
    pub id:         Uuid,
    pub return_id:  Uuid,
    pub product_id: Uuid,
    pub quantity:   i32,
    pub reason:     String,
    pub restock:    bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ReturnDto {
    pub id:             Uuid,
    pub sale_id:        Uuid,
    pub return_date:    DateTime<Utc>,
    pub employee_id:    Uuid,
    pub customer_id:    Option<Uuid>,
    pub total_refund:   Decimal,
    pub created_at:     DateTime<Utc>,
    pub return_details: Vec<ReturnDetailDto>,
}

pub fn map_return(ret: &Return) -> ReturnDto {
    ReturnDto {
        id:             ret.id.as_uuid(),
        sale_id:        ret.sale_id.as_uuid(),
        return_date:    ret.return_date,
        employee_id:    ret.employee_id,
        customer_id:    ret.customer_id,
        total_refund:   ret.total_refund,
        created_at:     ret.created_at,
        return_details: ret.return_details.iter().map(map_return_detail).collect(),
    }
}

pub fn map_return_detail(d: &ReturnDetail) -> ReturnDetailDto {
    ReturnDetailDto {
        id:         d.id.as_uuid(),
        return_id:  d.return_id.as_uuid(),
        product_id: d.product_id,
        quantity:   d.quantity,
        reason:     d.reason.as_str().to_string(),
        restock:    d.restock,
        created_at: d.created_at,
    }
}
