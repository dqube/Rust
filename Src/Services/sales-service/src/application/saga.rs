//! Order saga handlers — plain async functions called from NATS subscription loops in main.rs.
//! These are NOT registered with the Mediator; they run as background tasks.

use std::sync::Arc;

use chrono::Utc;
use ddd_shared_kernel::{AppError, OutboxMessage, OutboxRepository};
use tracing::{info, warn};

use crate::application::integration_events::*;
use crate::domain::entities::{OrderSaga, SagaOrderItem};
use crate::domain::enums::OrderSagaStep;
use crate::domain::ids::SaleId;
use crate::domain::repositories::{OrderSagaRepository, SaleRepository};

pub async fn handle_order_placed(
    evt:       OrderPlacedIntegrationEvent,
    saga_repo: &Arc<dyn OrderSagaRepository>,
    _sale_repo: &Arc<dyn SaleRepository>,
    _outbox:   &Arc<dyn OutboxRepository>,
) -> Result<(), AppError> {
    let order_id = SaleId::from_uuid(evt.order_id);
    info!(order_id = %order_id, "OrderSaga started.");
    let items: Vec<SagaOrderItem> = evt.items.iter().map(|i| SagaOrderItem {
        product_id: i.product_id,
        quantity:   i.quantity,
        unit_price: i.unit_price,
    }).collect();
    let mut saga = OrderSaga {
        order_id,
        order_number:   evt.order_number.clone(),
        customer_id:    evt.customer_id,
        store_id:       evt.store_id,
        total:          evt.total,
        reservation_id: None,
        payment_id:     None,
        step:           OrderSagaStep::WaitingForStockReservation,
        failure_reason: None,
        items,
        created_at:     Utc::now(),
        updated_at:     Utc::now(),
    };
    saga_repo.save(&mut saga).await?;
    Ok(())
}

pub async fn handle_stock_reserved(
    evt:       StockReservedIntegrationEvent,
    saga_repo: &Arc<dyn OrderSagaRepository>,
    _sale_repo: &Arc<dyn SaleRepository>,
    outbox:    &Arc<dyn OutboxRepository>,
) -> Result<(), AppError> {
    let order_id = SaleId::from_uuid(evt.order_id);
    let mut saga = match saga_repo.find_by_order_id(order_id).await? {
        Some(s) => s,
        None    => { warn!(order_id = %order_id, "OrderSaga not found for StockReserved."); return Ok(()); }
    };
    if saga.step != OrderSagaStep::WaitingForStockReservation { return Ok(()); }
    saga.reservation_id = Some(evt.reservation_id);
    saga.step           = OrderSagaStep::WaitingForPayment;
    saga.updated_at     = Utc::now();
    saga_repo.save(&mut saga).await?;
    let trigger = OrderRefundRequestedIntegrationEvent {
        order_id:     saga.order_id.as_uuid(),
        customer_id:  saga.customer_id,
        total_amount: saga.total,
        reason:       "payment-capture".into(),
    };
    let payload = serde_json::to_value(&trigger)
        .map_err(|e| AppError::internal(e.to_string()))?;
    outbox.save(&OutboxMessage::new(
        saga.order_id.to_string(), "Sale", "order.refund-requested",
        OrderRefundRequestedIntegrationEvent::TOPIC, payload,
    )).await?;
    info!(order_id = %order_id, "Stock reserved, requesting payment capture.");
    Ok(())
}

pub async fn handle_stock_reservation_failed(
    evt:       StockReservationFailedIntegrationEvent,
    saga_repo: &Arc<dyn OrderSagaRepository>,
    _sale_repo: &Arc<dyn SaleRepository>,
    outbox:    &Arc<dyn OutboxRepository>,
) -> Result<(), AppError> {
    let order_id = SaleId::from_uuid(evt.order_id);
    let mut saga = match saga_repo.find_by_order_id(order_id).await? {
        Some(s) => s,
        None    => { warn!(order_id = %order_id, "OrderSaga not found."); return Ok(()); }
    };
    if saga.step != OrderSagaStep::WaitingForStockReservation { return Ok(()); }
    let reason = format!("Stock reservation failed: {}", evt.reason);
    saga.step           = OrderSagaStep::Failed;
    saga.failure_reason = Some(reason.clone());
    saga.updated_at     = Utc::now();
    saga_repo.save(&mut saga).await?;
    publish_order_cancelled(&saga, reason, outbox).await?;
    Ok(())
}

pub async fn handle_payment_captured(
    evt:       PaymentCapturedIntegrationEvent,
    saga_repo: &Arc<dyn OrderSagaRepository>,
    _sale_repo: &Arc<dyn SaleRepository>,
    outbox:    &Arc<dyn OutboxRepository>,
) -> Result<(), AppError> {
    let order_id = SaleId::from_uuid(evt.order_id);
    let mut saga = match saga_repo.find_by_order_id(order_id).await? {
        Some(s) => s,
        None    => { warn!(order_id = %order_id, "OrderSaga not found."); return Ok(()); }
    };
    if saga.step != OrderSagaStep::WaitingForPayment { return Ok(()); }
    saga.payment_id = Some(evt.payment_id);
    saga.step       = OrderSagaStep::Confirmed;
    saga.updated_at = Utc::now();
    saga_repo.save(&mut saga).await?;
    let confirmed = OrderConfirmedIntegrationEvent {
        order_id:     saga.order_id.as_uuid(),
        order_number: saga.order_number.clone(),
        customer_id:  saga.customer_id,
        store_id:     saga.store_id,
        items:        saga.items.iter().map(|i| SagaItem { product_id: i.product_id, quantity: i.quantity, unit_price: i.unit_price }).collect(),
        total:        saga.total,
        confirmed_at: Utc::now().to_rfc3339(),
    };
    let payload = serde_json::to_value(&confirmed)
        .map_err(|e| AppError::internal(e.to_string()))?;
    outbox.save(&OutboxMessage::new(
        saga.order_id.to_string(), "Sale", "order.confirmed",
        OrderConfirmedIntegrationEvent::TOPIC, payload,
    )).await?;
    info!(order_id = %order_id, "Payment captured, order confirmed.");
    Ok(())
}

pub async fn handle_payment_failed(
    evt:       PaymentFailedIntegrationEvent,
    saga_repo: &Arc<dyn OrderSagaRepository>,
    _sale_repo: &Arc<dyn SaleRepository>,
    outbox:    &Arc<dyn OutboxRepository>,
) -> Result<(), AppError> {
    let order_id = SaleId::from_uuid(evt.order_id);
    let mut saga = match saga_repo.find_by_order_id(order_id).await? {
        Some(s) => s,
        None    => { warn!(order_id = %order_id, "OrderSaga not found."); return Ok(()); }
    };
    if saga.step != OrderSagaStep::WaitingForPayment { return Ok(()); }
    let reason = format!("Payment failed: {}", evt.reason);
    saga.step           = OrderSagaStep::Failed;
    saga.failure_reason = Some(reason.clone());
    saga.updated_at     = Utc::now();
    saga_repo.save(&mut saga).await?;
    publish_order_cancelled(&saga, reason, outbox).await?;
    Ok(())
}

pub async fn handle_payment_initiation_failed(
    evt:       PaymentInitiationFailedIntegrationEvent,
    saga_repo: &Arc<dyn OrderSagaRepository>,
    _sale_repo: &Arc<dyn SaleRepository>,
    outbox:    &Arc<dyn OutboxRepository>,
) -> Result<(), AppError> {
    let order_id = SaleId::from_uuid(evt.order_id);
    let mut saga = match saga_repo.find_by_order_id(order_id).await? {
        Some(s) => s,
        None    => { warn!(order_id = %order_id, "OrderSaga not found."); return Ok(()); }
    };
    if saga.step != OrderSagaStep::WaitingForPayment { return Ok(()); }
    let reason = format!("Payment initiation failed: {}", evt.reason);
    saga.step           = OrderSagaStep::Failed;
    saga.failure_reason = Some(reason.clone());
    saga.updated_at     = Utc::now();
    saga_repo.save(&mut saga).await?;
    publish_order_cancelled(&saga, reason, outbox).await?;
    Ok(())
}

pub async fn handle_payment_refunded(
    evt:       PaymentRefundedIntegrationEvent,
    _saga_repo: &Arc<dyn OrderSagaRepository>,
    sale_repo: &Arc<dyn SaleRepository>,
    outbox:    &Arc<dyn OutboxRepository>,
) -> Result<(), AppError> {
    let sale_id = SaleId::from_uuid(evt.sale_id);
    let mut sale = match sale_repo.find_by_id(sale_id).await? {
        Some(s) => s,
        None    => { info!(sale_id = %sale_id, "Sale not found for PaymentRefunded, skipping."); return Ok(()); }
    };
    let reason = format!("Refunded: {}", evt.reason);
    if sale.cancel(&reason).is_ok() {
        sale_repo.save(&mut sale).await?;
        let evt_out = SaleCancelledIntegrationEvent {
            sale_id:      sale.id.as_uuid(),
            reason,
            cancelled_at: Utc::now().to_rfc3339(),
        };
        let payload = serde_json::to_value(&evt_out)
            .map_err(|e| AppError::internal(e.to_string()))?;
        outbox.save(&OutboxMessage::new(
            sale.id.to_string(), "Sale", "sale.cancelled",
            SaleCancelledIntegrationEvent::TOPIC, payload,
        )).await?;
    }
    Ok(())
}

pub async fn handle_promotion_applied(
    evt:       PromotionAppliedIntegrationEvent,
    _saga_repo: &Arc<dyn OrderSagaRepository>,
    sale_repo: &Arc<dyn SaleRepository>,
    _outbox:   &Arc<dyn OutboxRepository>,
) -> Result<(), AppError> {
    let sale_id = SaleId::from_uuid(evt.sale_id);
    let mut sale = match sale_repo.find_with_details(sale_id).await? {
        Some(s) => s,
        None    => { info!(sale_id = %sale_id, "Sale not found for PromotionApplied, skipping."); return Ok(()); }
    };
    sale.apply_discount(None, evt.campaign_id, evt.rule_id, evt.discount_amount);
    sale_repo.save(&mut sale).await?;
    Ok(())
}

pub async fn handle_customer_deleted(
    evt:       CustomerDeletedIntegrationEvent,
    _saga_repo: &Arc<dyn OrderSagaRepository>,
    sale_repo: &Arc<dyn SaleRepository>,
    _outbox:   &Arc<dyn OutboxRepository>,
) -> Result<(), AppError> {
    let sales = sale_repo.get_by_customer(evt.customer_id).await?;
    for mut sale in sales {
        sale.anonymize_customer();
        if let Err(e) = sale_repo.save(&mut sale).await {
            tracing::error!(sale_id = %sale.id, "Failed to anonymize customer in sale: {e}");
        }
    }
    Ok(())
}

async fn publish_order_cancelled(
    saga:   &OrderSaga,
    reason: String,
    outbox: &Arc<dyn OutboxRepository>,
) -> Result<(), AppError> {
    let evt = OrderCancelledIntegrationEvent {
        order_id:     saga.order_id.as_uuid(),
        order_number: saga.order_number.clone(),
        customer_id:  saga.customer_id,
        store_id:     saga.store_id,
        items:        saga.items.iter().map(|i| SagaItem { product_id: i.product_id, quantity: i.quantity, unit_price: i.unit_price }).collect(),
        reason,
        cancelled_at: Utc::now().to_rfc3339(),
    };
    let payload = serde_json::to_value(&evt)
        .map_err(|e| AppError::internal(e.to_string()))?;
    outbox.save(&OutboxMessage::new(
        saga.order_id.to_string(), "Sale", "order.cancelled",
        OrderCancelledIntegrationEvent::TOPIC, payload,
    )).await?;
    Ok(())
}
