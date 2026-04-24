//! Sale command and query handlers, self-registered via Mediator.

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use ddd_application::{
    register_command_handler, register_query_handler, CommandHandler, QueryHandler,
};
use ddd_shared_kernel::{
    AppError, AppResult, BlobStorage, OutboxMessage, OutboxRepository,
};
use serde_json::json;
use uuid::Uuid;

use super::super::commands::{
    AddSaleDetail, ApplyDiscount, CancelSale, CompleteSale, CreateSale, PlaceOrder,
    RemoveSaleDetail, SetPaymentTransaction, SetSaleAddresses, UpdateSaleDetail,
    UpdateSaleStatus, UploadSaleReceipt,
};
use super::super::deps::AppDeps;
use super::super::dtos::{map_sale, SaleDto};
use super::super::integration_events::{
    OrderPlacedIntegrationEvent, SagaItem, SaleCancelledIntegrationEvent,
    SaleCompletedIntegrationEvent, SaleCreatedIntegrationEvent,
};
use super::super::queries::{
    GetSaleById, GetSaleByReceipt, GetSaleReceiptUrl, GetSales, GetSalesByCustomer,
    GetSalesByEmployee, GetSalesByStore,
};
use crate::domain::entities::Sale;
use crate::domain::enums::OrderStatus;
use crate::domain::repositories::{SaleRepository};

// ─── helpers ─────────────────────────────────────────────────────────────────

async fn append_outbox(
    outbox: &Arc<dyn OutboxRepository>,
    aggregate_id: Uuid,
    aggregate_type: &str,
    event_type: &str,
    subject: &str,
    payload: serde_json::Value,
) -> AppResult<()> {
    let msg = OutboxMessage::new(
        aggregate_id.to_string(),
        aggregate_type.to_string(),
        event_type.to_string(),
        subject.to_string(),
        payload,
    );
    outbox.save(&msg).await
}

async fn load_sale(repo: &Arc<dyn SaleRepository>, id: crate::domain::ids::SaleId) -> AppResult<Sale> {
    repo.find_with_details(id)
        .await?
        .ok_or_else(|| AppError::not_found("Sale", id.to_string()))
}

// ─── CreateSaleHandler ───────────────────────────────────────────────────────

pub struct CreateSaleHandler {
    repo:   Arc<dyn SaleRepository>,
    outbox: Arc<dyn OutboxRepository>,
}

#[async_trait]
impl CommandHandler<CreateSale> for CreateSaleHandler {
    async fn handle(&self, cmd: CreateSale) -> AppResult<SaleDto> {
        if self.repo.receipt_exists(&cmd.receipt_number).await? {
            return Err(AppError::conflict(format!(
                "Receipt number '{}' already exists.",
                cmd.receipt_number
            )));
        }
        let mut sale = Sale::create(
            cmd.store_id,
            cmd.employee_id,
            cmd.register_id,
            cmd.receipt_number,
            cmd.customer_id,
            cmd.channel,
        );
        self.repo.save(&mut sale).await?;

        let evt = SaleCreatedIntegrationEvent {
            sale_id:          sale.id.as_uuid(),
            store_id:         sale.store_id,
            employee_id:      sale.employee_id,
            customer_id:      sale.customer_id,
            total_amount:     sale.total_amount,
            transaction_time: sale.transaction_time.to_rfc3339(),
        };
        append_outbox(
            &self.outbox,
            sale.id.as_uuid(),
            "Sale",
            "SaleCreated",
            SaleCreatedIntegrationEvent::TOPIC,
            serde_json::to_value(&evt).map_err(|e| AppError::internal(e.to_string()))?,
        )
        .await?;

        Ok(map_sale(&sale))
    }
}

register_command_handler!(CreateSale, AppDeps, |d: &AppDeps| {
    CreateSaleHandler { repo: d.sale_repo.clone(), outbox: d.outbox.clone() }
});

// ─── AddSaleDetailHandler ────────────────────────────────────────────────────

pub struct AddSaleDetailHandler {
    repo: Arc<dyn SaleRepository>,
}

#[async_trait]
impl CommandHandler<AddSaleDetail> for AddSaleDetailHandler {
    async fn handle(&self, cmd: AddSaleDetail) -> AppResult<()> {
        let mut sale = load_sale(&self.repo, cmd.sale_id).await?;
        sale.add_detail(cmd.product_id, cmd.variant_id, cmd.quantity, cmd.unit_price, cmd.tax_applied);
        self.repo.save(&mut sale).await
    }
}

register_command_handler!(AddSaleDetail, AppDeps, |d: &AppDeps| {
    AddSaleDetailHandler { repo: d.sale_repo.clone() }
});

// ─── UpdateSaleDetailHandler ─────────────────────────────────────────────────

pub struct UpdateSaleDetailHandler {
    repo: Arc<dyn SaleRepository>,
}

#[async_trait]
impl CommandHandler<UpdateSaleDetail> for UpdateSaleDetailHandler {
    async fn handle(&self, cmd: UpdateSaleDetail) -> AppResult<()> {
        let mut sale = load_sale(&self.repo, cmd.sale_id).await?;
        sale.update_detail(cmd.sale_detail_id, cmd.quantity, cmd.unit_price, cmd.tax_applied)
            .map_err(|e| AppError::validation("sale_detail", e))?;
        self.repo.save(&mut sale).await
    }
}

register_command_handler!(UpdateSaleDetail, AppDeps, |d: &AppDeps| {
    UpdateSaleDetailHandler { repo: d.sale_repo.clone() }
});

// ─── RemoveSaleDetailHandler ─────────────────────────────────────────────────

pub struct RemoveSaleDetailHandler {
    repo: Arc<dyn SaleRepository>,
}

#[async_trait]
impl CommandHandler<RemoveSaleDetail> for RemoveSaleDetailHandler {
    async fn handle(&self, cmd: RemoveSaleDetail) -> AppResult<()> {
        let mut sale = load_sale(&self.repo, cmd.sale_id).await?;
        sale.remove_detail(cmd.sale_detail_id).map_err(|e| AppError::validation("sale_detail", e))?;
        self.repo.save(&mut sale).await
    }
}

register_command_handler!(RemoveSaleDetail, AppDeps, |d: &AppDeps| {
    RemoveSaleDetailHandler { repo: d.sale_repo.clone() }
});

// ─── ApplyDiscountHandler ────────────────────────────────────────────────────

pub struct ApplyDiscountHandler {
    repo: Arc<dyn SaleRepository>,
}

#[async_trait]
impl CommandHandler<ApplyDiscount> for ApplyDiscountHandler {
    async fn handle(&self, cmd: ApplyDiscount) -> AppResult<()> {
        let mut sale = load_sale(&self.repo, cmd.sale_id).await?;
        sale.apply_discount(cmd.sale_detail_id, cmd.campaign_id, cmd.rule_id, cmd.discount_amount);
        self.repo.save(&mut sale).await
    }
}

register_command_handler!(ApplyDiscount, AppDeps, |d: &AppDeps| {
    ApplyDiscountHandler { repo: d.sale_repo.clone() }
});

// ─── CompleteSaleHandler ─────────────────────────────────────────────────────

pub struct CompleteSaleHandler {
    repo:   Arc<dyn SaleRepository>,
    outbox: Arc<dyn OutboxRepository>,
}

#[async_trait]
impl CommandHandler<CompleteSale> for CompleteSaleHandler {
    async fn handle(&self, cmd: CompleteSale) -> AppResult<()> {
        let mut sale = load_sale(&self.repo, cmd.sale_id).await?;
        sale.complete();
        self.repo.save(&mut sale).await?;

        let evt = SaleCompletedIntegrationEvent {
            sale_id:          sale.id.as_uuid(),
            total_amount:     sale.total_amount,
            transaction_time: sale.transaction_time.to_rfc3339(),
        };
        append_outbox(
            &self.outbox,
            sale.id.as_uuid(),
            "Sale",
            "SaleCompleted",
            SaleCompletedIntegrationEvent::TOPIC,
            serde_json::to_value(&evt).map_err(|e| AppError::internal(e.to_string()))?,
        )
        .await
    }
}

register_command_handler!(CompleteSale, AppDeps, |d: &AppDeps| {
    CompleteSaleHandler { repo: d.sale_repo.clone(), outbox: d.outbox.clone() }
});

// ─── CancelSaleHandler ───────────────────────────────────────────────────────

pub struct CancelSaleHandler {
    repo:   Arc<dyn SaleRepository>,
    outbox: Arc<dyn OutboxRepository>,
}

#[async_trait]
impl CommandHandler<CancelSale> for CancelSaleHandler {
    async fn handle(&self, cmd: CancelSale) -> AppResult<()> {
        let mut sale = load_sale(&self.repo, cmd.sale_id).await?;
        sale.cancel(&cmd.reason).map_err(|e| AppError::validation("sale", e))?;
        self.repo.save(&mut sale).await?;

        let evt = SaleCancelledIntegrationEvent {
            sale_id:      sale.id.as_uuid(),
            reason:       cmd.reason,
            cancelled_at: chrono::Utc::now().to_rfc3339(),
        };
        append_outbox(
            &self.outbox,
            sale.id.as_uuid(),
            "Sale",
            "SaleCancelled",
            SaleCancelledIntegrationEvent::TOPIC,
            serde_json::to_value(&evt).map_err(|e| AppError::internal(e.to_string()))?,
        )
        .await
    }
}

register_command_handler!(CancelSale, AppDeps, |d: &AppDeps| {
    CancelSaleHandler { repo: d.sale_repo.clone(), outbox: d.outbox.clone() }
});

// ─── UpdateSaleStatusHandler ─────────────────────────────────────────────────

pub struct UpdateSaleStatusHandler {
    repo: Arc<dyn SaleRepository>,
}

#[async_trait]
impl CommandHandler<UpdateSaleStatus> for UpdateSaleStatusHandler {
    async fn handle(&self, cmd: UpdateSaleStatus) -> AppResult<()> {
        let mut sale = load_sale(&self.repo, cmd.sale_id).await?;
        sale.status = OrderStatus::from_str(&cmd.status).unwrap();
        self.repo.save(&mut sale).await
    }
}

register_command_handler!(UpdateSaleStatus, AppDeps, |d: &AppDeps| {
    UpdateSaleStatusHandler { repo: d.sale_repo.clone() }
});

// ─── SetSaleAddressesHandler ─────────────────────────────────────────────────

pub struct SetSaleAddressesHandler {
    repo: Arc<dyn SaleRepository>,
}

#[async_trait]
impl CommandHandler<SetSaleAddresses> for SetSaleAddressesHandler {
    async fn handle(&self, cmd: SetSaleAddresses) -> AppResult<()> {
        let mut sale = load_sale(&self.repo, cmd.sale_id).await?;
        sale.set_addresses(cmd.shipping_address.into(), cmd.billing_address.into());
        self.repo.save(&mut sale).await
    }
}

register_command_handler!(SetSaleAddresses, AppDeps, |d: &AppDeps| {
    SetSaleAddressesHandler { repo: d.sale_repo.clone() }
});

// ─── SetPaymentTransactionHandler ────────────────────────────────────────────

pub struct SetPaymentTransactionHandler {
    repo: Arc<dyn SaleRepository>,
}

#[async_trait]
impl CommandHandler<SetPaymentTransaction> for SetPaymentTransactionHandler {
    async fn handle(&self, cmd: SetPaymentTransaction) -> AppResult<()> {
        let mut sale = load_sale(&self.repo, cmd.sale_id).await?;
        sale.set_payment_transaction(cmd.transaction_id);
        self.repo.save(&mut sale).await
    }
}

register_command_handler!(SetPaymentTransaction, AppDeps, |d: &AppDeps| {
    SetPaymentTransactionHandler { repo: d.sale_repo.clone() }
});

// ─── UploadSaleReceiptHandler ────────────────────────────────────────────────

pub struct UploadSaleReceiptHandler {
    repo:             Arc<dyn SaleRepository>,
    storage:          Arc<dyn BlobStorage>,
    bucket:           String,
    presign_ttl_secs: u64,
}

#[async_trait]
impl CommandHandler<UploadSaleReceipt> for UploadSaleReceiptHandler {
    async fn handle(&self, cmd: UploadSaleReceipt) -> AppResult<String> {
        let mut sale = load_sale(&self.repo, cmd.sale_id).await?;
        let ext = std::path::Path::new(&cmd.file_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("bin");
        let object_name = format!("receipts/{}/{}.{}", sale.id.as_uuid(), Uuid::new_v4(), ext);

        self.storage
            .upload(&self.bucket, &object_name, &cmd.content_type, cmd.file_content)
            .await?;

        sale.set_receipt_object_name(object_name.clone());
        self.repo.save(&mut sale).await?;

        let presigned = self
            .storage
            .presigned_get(&self.bucket, &object_name, Duration::from_secs(self.presign_ttl_secs))
            .await?;
        Ok(presigned.url)
    }
}

register_command_handler!(UploadSaleReceipt, AppDeps, |d: &AppDeps| {
    UploadSaleReceiptHandler {
        repo:             d.sale_repo.clone(),
        storage:          d.blob_storage.clone(),
        bucket:           d.blob_bucket.clone(),
        presign_ttl_secs: d.presign_ttl_secs,
    }
});

// ─── PlaceOrderHandler ───────────────────────────────────────────────────────

pub struct PlaceOrderHandler {
    repo:   Arc<dyn SaleRepository>,
    outbox: Arc<dyn OutboxRepository>,
}

#[async_trait]
impl CommandHandler<PlaceOrder> for PlaceOrderHandler {
    async fn handle(&self, cmd: PlaceOrder) -> AppResult<SaleDto> {
        let mut sale = Sale::place_online_order(cmd.customer_id, cmd.store_id, cmd.items);
        self.repo.save(&mut sale).await?;

        let evt = OrderPlacedIntegrationEvent {
            order_id:     sale.id.as_uuid(),
            order_number: sale.receipt_number.clone(),
            customer_id:  cmd.customer_id,
            store_id:     cmd.store_id,
            sub_total:    sale.sub_total,
            tax:          sale.tax_amount,
            total:        sale.total_amount,
            currency:     cmd.currency,
            items:        sale.sale_details.iter().map(|d| SagaItem {
                product_id: d.product_id,
                quantity:   d.quantity,
                unit_price: d.unit_price,
            }).collect(),
            placed_at:    sale.transaction_time.to_rfc3339(),
        };
        append_outbox(
            &self.outbox,
            sale.id.as_uuid(),
            "Sale",
            "OrderPlaced",
            OrderPlacedIntegrationEvent::TOPIC,
            serde_json::to_value(&evt).map_err(|e| AppError::internal(e.to_string()))?,
        )
        .await?;

        Ok(map_sale(&sale))
    }
}

register_command_handler!(PlaceOrder, AppDeps, |d: &AppDeps| {
    PlaceOrderHandler { repo: d.sale_repo.clone(), outbox: d.outbox.clone() }
});

// ─── Query handlers ──────────────────────────────────────────────────────────

pub struct GetSaleByIdHandler { repo: Arc<dyn SaleRepository> }

#[async_trait]
impl QueryHandler<GetSaleById> for GetSaleByIdHandler {
    async fn handle(&self, q: GetSaleById) -> AppResult<Option<SaleDto>> {
        Ok(self.repo.find_with_details(q.sale_id).await?.as_ref().map(map_sale))
    }
}

register_query_handler!(GetSaleById, AppDeps, |d: &AppDeps| {
    GetSaleByIdHandler { repo: d.sale_repo.clone() }
});

pub struct GetSaleByReceiptHandler { repo: Arc<dyn SaleRepository> }

#[async_trait]
impl QueryHandler<GetSaleByReceipt> for GetSaleByReceiptHandler {
    async fn handle(&self, q: GetSaleByReceipt) -> AppResult<Option<SaleDto>> {
        Ok(self.repo.find_by_receipt(&q.receipt_number).await?.as_ref().map(map_sale))
    }
}

register_query_handler!(GetSaleByReceipt, AppDeps, |d: &AppDeps| {
    GetSaleByReceiptHandler { repo: d.sale_repo.clone() }
});

pub struct GetSalesHandler { repo: Arc<dyn SaleRepository> }

#[async_trait]
impl QueryHandler<GetSales> for GetSalesHandler {
    async fn handle(&self, q: GetSales) -> AppResult<(Vec<SaleDto>, u64)> {
        let (rows, total) = self.repo.get_all(q.page, q.page_size, q.status).await?;
        Ok((rows.iter().map(map_sale).collect(), total))
    }
}

register_query_handler!(GetSales, AppDeps, |d: &AppDeps| {
    GetSalesHandler { repo: d.sale_repo.clone() }
});

pub struct GetSalesByStoreHandler { repo: Arc<dyn SaleRepository> }

#[async_trait]
impl QueryHandler<GetSalesByStore> for GetSalesByStoreHandler {
    async fn handle(&self, q: GetSalesByStore) -> AppResult<Vec<SaleDto>> {
        Ok(self.repo.get_by_store(q.store_id, q.from_date, q.to_date).await?.iter().map(map_sale).collect())
    }
}

register_query_handler!(GetSalesByStore, AppDeps, |d: &AppDeps| {
    GetSalesByStoreHandler { repo: d.sale_repo.clone() }
});

pub struct GetSalesByEmployeeHandler { repo: Arc<dyn SaleRepository> }

#[async_trait]
impl QueryHandler<GetSalesByEmployee> for GetSalesByEmployeeHandler {
    async fn handle(&self, q: GetSalesByEmployee) -> AppResult<Vec<SaleDto>> {
        Ok(self.repo.get_by_employee(q.employee_id, q.from_date, q.to_date).await?.iter().map(map_sale).collect())
    }
}

register_query_handler!(GetSalesByEmployee, AppDeps, |d: &AppDeps| {
    GetSalesByEmployeeHandler { repo: d.sale_repo.clone() }
});

pub struct GetSalesByCustomerHandler { repo: Arc<dyn SaleRepository> }

#[async_trait]
impl QueryHandler<GetSalesByCustomer> for GetSalesByCustomerHandler {
    async fn handle(&self, q: GetSalesByCustomer) -> AppResult<Vec<SaleDto>> {
        Ok(self.repo.get_by_customer(q.customer_id).await?.iter().map(map_sale).collect())
    }
}

register_query_handler!(GetSalesByCustomer, AppDeps, |d: &AppDeps| {
    GetSalesByCustomerHandler { repo: d.sale_repo.clone() }
});

pub struct GetSaleReceiptUrlHandler {
    repo:             Arc<dyn SaleRepository>,
    storage:          Arc<dyn BlobStorage>,
    bucket:           String,
    presign_ttl_secs: u64,
}

#[async_trait]
impl QueryHandler<GetSaleReceiptUrl> for GetSaleReceiptUrlHandler {
    async fn handle(&self, q: GetSaleReceiptUrl) -> AppResult<Option<String>> {
        let sale = self.repo.find_by_id(q.sale_id).await?;
        let Some(sale) = sale else { return Ok(None); };
        let Some(object_name) = sale.receipt_object_name.as_deref() else { return Ok(None); };
        let presigned = self
            .storage
            .presigned_get(&self.bucket, object_name, Duration::from_secs(self.presign_ttl_secs))
            .await?;
        // touch json! to keep import in case of future evolution; otherwise unused
        let _ = json!(null);
        Ok(Some(presigned.url))
    }
}

register_query_handler!(GetSaleReceiptUrl, AppDeps, |d: &AppDeps| {
    GetSaleReceiptUrlHandler {
        repo:             d.sale_repo.clone(),
        storage:          d.blob_storage.clone(),
        bucket:           d.blob_bucket.clone(),
        presign_ttl_secs: d.presign_ttl_secs,
    }
});
