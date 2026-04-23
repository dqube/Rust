#![allow(unused_imports)]
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use std::sync::Arc;
use ddd_shared_kernel::AppError;
use uuid::Uuid;

use crate::domain::{
    entities::{Address, AppliedDiscount, OrderSaga, Return, ReturnDetail, Sale, SaleDetail, SagaOrderItem},
    enums::{OrderSagaStep, OrderStatus, ReturnReason, SalesChannel},
    ids::{AppliedDiscountId, ReturnDetailId, ReturnId, SaleDetailId, SaleId},
    repositories::{OrderSagaRepository, ReturnRepository, SaleRepository},
};
use super::models::*;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn db_err(e: sea_orm::DbErr) -> AppError {
    AppError::database(e.to_string())
}

fn json_err(e: serde_json::Error) -> AppError {
    AppError::internal(e.to_string())
}

fn to_utc(dt: sea_orm::prelude::DateTimeWithTimeZone) -> DateTime<Utc> {
    use chrono::TimeZone;
    dt.with_timezone(&Utc)
}

fn from_utc(dt: DateTime<Utc>) -> sea_orm::prelude::DateTimeWithTimeZone {
    dt.fixed_offset()
}

// ── Model → domain mappers ────────────────────────────────────────────────────

fn model_to_sale_detail(m: sale_detail::Model) -> SaleDetail {
    SaleDetail {
        id:               SaleDetailId(m.id),
        sale_id:          SaleId(m.sale_id),
        product_id:       m.product_id,
        variant_id:       m.variant_id,
        quantity:         m.quantity,
        unit_price:       m.unit_price,
        applied_discount: m.applied_discount,
        tax_applied:      m.tax_applied,
        line_total:       m.line_total,
        created_at:       to_utc(m.created_at),
    }
}

fn model_to_applied_discount(m: applied_discount::Model) -> AppliedDiscount {
    AppliedDiscount {
        id:              AppliedDiscountId(m.id),
        sale_id:         SaleId(m.sale_id),
        sale_detail_id:  m.sale_detail_id.map(SaleDetailId),
        campaign_id:     m.campaign_id,
        rule_id:         m.rule_id,
        discount_amount: m.discount_amount,
        created_at:      to_utc(m.created_at),
    }
}

fn model_to_sale(m: sale::Model, details: Vec<SaleDetail>, discounts: Vec<AppliedDiscount>) -> Sale {
    let shipping_address = m.shipping_address
        .and_then(|v| serde_json::from_value::<Address>(v).ok());
    let billing_address = m.billing_address
        .and_then(|v| serde_json::from_value::<Address>(v).ok());
    Sale {
        id:                     SaleId(m.id),
        store_id:               m.store_id,
        employee_id:            m.employee_id,
        customer_id:            m.customer_id,
        register_id:            m.register_id,
        receipt_number:         m.receipt_number,
        transaction_time:       to_utc(m.transaction_time),
        sub_total:              m.sub_total,
        discount_total:         m.discount_total,
        tax_amount:             m.tax_amount,
        total_amount:           m.total_amount,
        channel:                SalesChannel::from_str(&m.channel),
        status:                 OrderStatus::from_str(&m.status),
        shipping_address,
        billing_address,
        payment_transaction_id: m.payment_transaction_id,
        receipt_object_name:    m.receipt_object_name,
        created_at:             to_utc(m.created_at),
        sale_details:           details,
        applied_discounts:      discounts,
    }
}

fn model_to_return_detail(m: return_detail::Model) -> ReturnDetail {
    ReturnDetail {
        id:         ReturnDetailId(m.id),
        return_id:  ReturnId(m.return_id),
        product_id: m.product_id,
        quantity:   m.quantity,
        reason:     ReturnReason::from_str(&m.reason),
        restock:    m.restock,
        created_at: to_utc(m.created_at),
    }
}

fn model_to_return(m: return_entity::Model, details: Vec<ReturnDetail>) -> Return {
    Return {
        id:             ReturnId(m.id),
        sale_id:        SaleId(m.sale_id),
        return_date:    to_utc(m.return_date),
        employee_id:    m.employee_id,
        customer_id:    m.customer_id,
        total_refund:   m.total_refund,
        created_at:     to_utc(m.created_at),
        return_details: details,
    }
}

fn model_to_saga(m: order_saga::Model) -> OrderSaga {
    let items: Vec<SagaOrderItem> = serde_json::from_value(m.items).unwrap_or_default();
    OrderSaga {
        order_id:       SaleId(m.order_id),
        order_number:   m.order_number,
        customer_id:    m.customer_id,
        store_id:       m.store_id,
        total:          m.total,
        reservation_id: m.reservation_id,
        payment_id:     m.payment_id,
        step:           OrderSagaStep::from_str(&m.step),
        failure_reason: m.failure_reason,
        items,
        created_at:     to_utc(m.created_at),
        updated_at:     to_utc(m.updated_at),
    }
}

// ── PgSaleRepository ──────────────────────────────────────────────────────────

pub struct PgSaleRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl SaleRepository for PgSaleRepository {
    async fn find_by_id(&self, id: SaleId) -> Result<Option<Sale>, AppError> {
        let Some(m) = sale::Entity::find_by_id(id.0)
            .one(&*self.0).await.map_err(db_err)? else { return Ok(None); };
        Ok(Some(model_to_sale(m, vec![], vec![])))
    }

    async fn find_with_details(&self, id: SaleId) -> Result<Option<Sale>, AppError> {
        let Some(m) = sale::Entity::find_by_id(id.0)
            .one(&*self.0).await.map_err(db_err)? else { return Ok(None); };
        let details = sale_detail::Entity::find()
            .filter(sale_detail::Column::SaleId.eq(id.0))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(model_to_sale_detail).collect();
        let discounts = applied_discount::Entity::find()
            .filter(applied_discount::Column::SaleId.eq(id.0))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(model_to_applied_discount).collect();
        Ok(Some(model_to_sale(m, details, discounts)))
    }

    async fn find_by_receipt(&self, receipt: &str) -> Result<Option<Sale>, AppError> {
        let Some(m) = sale::Entity::find()
            .filter(sale::Column::ReceiptNumber.eq(receipt.to_string()))
            .one(&*self.0).await.map_err(db_err)? else { return Ok(None); };
        let id = SaleId(m.id);
        let details = sale_detail::Entity::find()
            .filter(sale_detail::Column::SaleId.eq(id.0))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(model_to_sale_detail).collect();
        let discounts = applied_discount::Entity::find()
            .filter(applied_discount::Column::SaleId.eq(id.0))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(model_to_applied_discount).collect();
        Ok(Some(model_to_sale(m, details, discounts)))
    }

    async fn receipt_exists(&self, receipt: &str) -> Result<bool, AppError> {
        let count = sale::Entity::find()
            .filter(sale::Column::ReceiptNumber.eq(receipt.to_string()))
            .count(&*self.0).await.map_err(db_err)?;
        Ok(count > 0)
    }

    async fn get_all(&self, page: i32, page_size: i32, status: Option<String>) -> Result<(Vec<Sale>, u64), AppError> {
        let mut q = sale::Entity::find();
        if let Some(s) = status {
            if !s.is_empty() {
                q = q.filter(sale::Column::Status.eq(s));
            }
        }
        let paginator = q.order_by_desc(sale::Column::CreatedAt)
            .paginate(&*self.0, page_size.max(1) as u64);
        let total  = paginator.num_items().await.map_err(db_err)?;
        let models = paginator.fetch_page((page.max(1) - 1) as u64).await.map_err(db_err)?;
        Ok((models.into_iter().map(|m| model_to_sale(m, vec![], vec![])).collect(), total))
    }

    async fn get_by_store(&self, store_id: i32, from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> Result<Vec<Sale>, AppError> {
        let mut q = sale::Entity::find().filter(sale::Column::StoreId.eq(store_id));
        if let Some(f) = from { q = q.filter(sale::Column::CreatedAt.gte(f.fixed_offset())); }
        if let Some(t) = to   { q = q.filter(sale::Column::CreatedAt.lte(t.fixed_offset())); }
        let models = q.order_by_desc(sale::Column::CreatedAt).all(&*self.0).await.map_err(db_err)?;
        Ok(models.into_iter().map(|m| model_to_sale(m, vec![], vec![])).collect())
    }

    async fn get_by_employee(&self, employee_id: Uuid, from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> Result<Vec<Sale>, AppError> {
        let mut q = sale::Entity::find().filter(sale::Column::EmployeeId.eq(employee_id));
        if let Some(f) = from { q = q.filter(sale::Column::CreatedAt.gte(f.fixed_offset())); }
        if let Some(t) = to   { q = q.filter(sale::Column::CreatedAt.lte(t.fixed_offset())); }
        let models = q.order_by_desc(sale::Column::CreatedAt).all(&*self.0).await.map_err(db_err)?;
        Ok(models.into_iter().map(|m| model_to_sale(m, vec![], vec![])).collect())
    }

    async fn get_by_customer(&self, customer_id: Uuid) -> Result<Vec<Sale>, AppError> {
        let models = sale::Entity::find()
            .filter(sale::Column::CustomerId.eq(customer_id))
            .order_by_desc(sale::Column::CreatedAt)
            .all(&*self.0).await.map_err(db_err)?;
        Ok(models.into_iter().map(|m| model_to_sale(m, vec![], vec![])).collect())
    }

    async fn save(&self, sale: &mut Sale) -> Result<(), AppError> {
        let shipping_json = sale.shipping_address.as_ref()
            .map(|a| serde_json::to_value(a).map_err(json_err)).transpose()?;
        let billing_json = sale.billing_address.as_ref()
            .map(|a| serde_json::to_value(a).map_err(json_err)).transpose()?;

        let active = sale::ActiveModel {
            id:                     Set(sale.id.0),
            store_id:               Set(sale.store_id),
            employee_id:            Set(sale.employee_id),
            customer_id:            Set(sale.customer_id),
            register_id:            Set(sale.register_id),
            receipt_number:         Set(sale.receipt_number.clone()),
            transaction_time:       Set(from_utc(sale.transaction_time)),
            sub_total:              Set(sale.sub_total),
            discount_total:         Set(sale.discount_total),
            tax_amount:             Set(sale.tax_amount),
            total_amount:           Set(sale.total_amount),
            channel:                Set(sale.channel.as_str().to_string()),
            status:                 Set(sale.status.as_str().to_string()),
            shipping_address:       Set(shipping_json),
            billing_address:        Set(billing_json),
            payment_transaction_id: Set(sale.payment_transaction_id.clone()),
            receipt_object_name:    Set(sale.receipt_object_name.clone()),
            created_at:             Set(from_utc(sale.created_at)),
        };
        sale::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(sale::Column::Id)
                    .update_columns([
                        sale::Column::CustomerId,
                        sale::Column::SubTotal,
                        sale::Column::DiscountTotal,
                        sale::Column::TaxAmount,
                        sale::Column::TotalAmount,
                        sale::Column::Status,
                        sale::Column::ShippingAddress,
                        sale::Column::BillingAddress,
                        sale::Column::PaymentTransactionId,
                        sale::Column::ReceiptObjectName,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;

        // Replace children
        applied_discount::Entity::delete_many()
            .filter(applied_discount::Column::SaleId.eq(sale.id.0))
            .exec(&*self.0).await.map_err(db_err)?;
        sale_detail::Entity::delete_many()
            .filter(sale_detail::Column::SaleId.eq(sale.id.0))
            .exec(&*self.0).await.map_err(db_err)?;

        if !sale.sale_details.is_empty() {
            let models: Vec<sale_detail::ActiveModel> = sale.sale_details.iter().map(|d| {
                sale_detail::ActiveModel {
                    id:               Set(d.id.0),
                    sale_id:          Set(d.sale_id.0),
                    product_id:       Set(d.product_id),
                    variant_id:       Set(d.variant_id),
                    quantity:         Set(d.quantity),
                    unit_price:       Set(d.unit_price),
                    applied_discount: Set(d.applied_discount),
                    tax_applied:      Set(d.tax_applied),
                    line_total:       Set(d.line_total),
                    created_at:       Set(from_utc(d.created_at)),
                }
            }).collect();
            sale_detail::Entity::insert_many(models).exec(&*self.0).await.map_err(db_err)?;
        }

        if !sale.applied_discounts.is_empty() {
            let models: Vec<applied_discount::ActiveModel> = sale.applied_discounts.iter().map(|d| {
                applied_discount::ActiveModel {
                    id:              Set(d.id.0),
                    sale_id:         Set(d.sale_id.0),
                    sale_detail_id:  Set(d.sale_detail_id.map(|i| i.0)),
                    campaign_id:     Set(d.campaign_id),
                    rule_id:         Set(d.rule_id),
                    discount_amount: Set(d.discount_amount),
                    created_at:      Set(from_utc(d.created_at)),
                }
            }).collect();
            applied_discount::Entity::insert_many(models).exec(&*self.0).await.map_err(db_err)?;
        }

        Ok(())
    }
}

// ── PgReturnRepository ────────────────────────────────────────────────────────

pub struct PgReturnRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl ReturnRepository for PgReturnRepository {
    async fn find_by_id(&self, id: ReturnId) -> Result<Option<Return>, AppError> {
        let Some(m) = return_entity::Entity::find_by_id(id.0)
            .one(&*self.0).await.map_err(db_err)? else { return Ok(None); };
        Ok(Some(model_to_return(m, vec![])))
    }

    async fn find_with_details(&self, id: ReturnId) -> Result<Option<Return>, AppError> {
        let Some(m) = return_entity::Entity::find_by_id(id.0)
            .one(&*self.0).await.map_err(db_err)? else { return Ok(None); };
        let details = return_detail::Entity::find()
            .filter(return_detail::Column::ReturnId.eq(id.0))
            .all(&*self.0).await.map_err(db_err)?
            .into_iter().map(model_to_return_detail).collect();
        Ok(Some(model_to_return(m, details)))
    }

    async fn get_by_sale(&self, sale_id: SaleId) -> Result<Vec<Return>, AppError> {
        let models = return_entity::Entity::find()
            .filter(return_entity::Column::SaleId.eq(sale_id.0))
            .all(&*self.0).await.map_err(db_err)?;
        Ok(models.into_iter().map(|m| model_to_return(m, vec![])).collect())
    }

    async fn get_by_employee(&self, employee_id: Uuid, from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> Result<Vec<Return>, AppError> {
        let mut q = return_entity::Entity::find()
            .filter(return_entity::Column::EmployeeId.eq(employee_id));
        if let Some(f) = from { q = q.filter(return_entity::Column::ReturnDate.gte(f.fixed_offset())); }
        if let Some(t) = to   { q = q.filter(return_entity::Column::ReturnDate.lte(t.fixed_offset())); }
        let models = q.order_by_desc(return_entity::Column::ReturnDate).all(&*self.0).await.map_err(db_err)?;
        Ok(models.into_iter().map(|m| model_to_return(m, vec![])).collect())
    }

    async fn get_by_customer(&self, customer_id: Uuid) -> Result<Vec<Return>, AppError> {
        let models = return_entity::Entity::find()
            .filter(return_entity::Column::CustomerId.eq(customer_id))
            .order_by_desc(return_entity::Column::ReturnDate)
            .all(&*self.0).await.map_err(db_err)?;
        Ok(models.into_iter().map(|m| model_to_return(m, vec![])).collect())
    }

    async fn save(&self, ret: &mut Return) -> Result<(), AppError> {
        let active = return_entity::ActiveModel {
            id:           Set(ret.id.0),
            sale_id:      Set(ret.sale_id.0),
            return_date:  Set(from_utc(ret.return_date)),
            employee_id:  Set(ret.employee_id),
            customer_id:  Set(ret.customer_id),
            total_refund: Set(ret.total_refund),
            created_at:   Set(from_utc(ret.created_at)),
        };
        return_entity::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(return_entity::Column::Id)
                    .update_columns([
                        return_entity::Column::TotalRefund,
                        return_entity::Column::CustomerId,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;

        return_detail::Entity::delete_many()
            .filter(return_detail::Column::ReturnId.eq(ret.id.0))
            .exec(&*self.0).await.map_err(db_err)?;

        if !ret.return_details.is_empty() {
            let models: Vec<return_detail::ActiveModel> = ret.return_details.iter().map(|d| {
                return_detail::ActiveModel {
                    id:         Set(d.id.0),
                    return_id:  Set(d.return_id.0),
                    product_id: Set(d.product_id),
                    quantity:   Set(d.quantity),
                    reason:     Set(d.reason.as_str().to_string()),
                    restock:    Set(d.restock),
                    created_at: Set(from_utc(d.created_at)),
                }
            }).collect();
            return_detail::Entity::insert_many(models).exec(&*self.0).await.map_err(db_err)?;
        }

        Ok(())
    }
}

// ── PgOrderSagaRepository ─────────────────────────────────────────────────────

pub struct PgOrderSagaRepository(pub Arc<DatabaseConnection>);

#[async_trait]
impl OrderSagaRepository for PgOrderSagaRepository {
    async fn find_by_order_id(&self, order_id: SaleId) -> Result<Option<OrderSaga>, AppError> {
        Ok(order_saga::Entity::find_by_id(order_id.0)
            .one(&*self.0).await.map_err(db_err)?
            .map(model_to_saga))
    }

    async fn save(&self, saga: &mut OrderSaga) -> Result<(), AppError> {
        let items_json = serde_json::to_value(&saga.items).map_err(json_err)?;
        let active = order_saga::ActiveModel {
            order_id:       Set(saga.order_id.0),
            order_number:   Set(saga.order_number.clone()),
            customer_id:    Set(saga.customer_id),
            store_id:       Set(saga.store_id),
            total:          Set(saga.total),
            reservation_id: Set(saga.reservation_id),
            payment_id:     Set(saga.payment_id),
            step:           Set(saga.step.as_str().to_string()),
            failure_reason: Set(saga.failure_reason.clone()),
            items:          Set(items_json),
            created_at:     Set(from_utc(saga.created_at)),
            updated_at:     Set(from_utc(saga.updated_at)),
        };
        order_saga::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(order_saga::Column::OrderId)
                    .update_columns([
                        order_saga::Column::ReservationId,
                        order_saga::Column::PaymentId,
                        order_saga::Column::Step,
                        order_saga::Column::FailureReason,
                        order_saga::Column::Items,
                        order_saga::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&*self.0).await.map_err(db_err)?;
        Ok(())
    }
}
