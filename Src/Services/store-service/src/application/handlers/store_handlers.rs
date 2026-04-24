use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use ddd_application::{register_command_handler, register_query_handler, CommandHandler, QueryHandler};
use ddd_shared_kernel::{AppError, AppResult, BlobStorage, OutboxMessage, OutboxRepository};
use tracing::info;

use crate::application::commands::*;
use crate::application::deps::AppDeps;
use crate::application::dtos::{map_store, PagedStoreDto, StoreDto};
use crate::application::integration_events::*;
use crate::application::queries::*;
use crate::domain::entities::Store;
use crate::domain::repositories::StoreRepository;

fn outbox_msg(id: i32, topic: &'static str, payload: serde_json::Value) -> OutboxMessage {
    OutboxMessage::new(id.to_string(), "Store", topic, topic, payload)
}

// ── CreateStoreHandler ────────────────────────────────────────────────────────

pub struct CreateStoreHandler {
    store_repo: Arc<dyn StoreRepository>,
    outbox:     Arc<dyn OutboxRepository>,
}

#[async_trait]
impl CommandHandler<CreateStore> for CreateStoreHandler {
    async fn handle(&self, cmd: CreateStore) -> AppResult<StoreDto> {
        if self.store_repo.name_exists(&cmd.name).await? {
            return Err(AppError::conflict(format!("Store name '{}' already exists.", cmd.name)));
        }
        let mut store = Store::create(
            cmd.name, cmd.location_id,
            cmd.address_street, cmd.address_city, cmd.address_postal_code, cmd.address_country,
            cmd.phone, cmd.geo_latitude, cmd.geo_longitude, cmd.schedules, cmd.services,
        )?;
        self.store_repo.save(&mut store).await?;
        store.emit_created();

        let evt = StoreCreatedIntegrationEvent {
            store_id: store.id.0,
            name:     store.name.clone(),
            city:     store.address_city.clone(),
            status:   store.status.as_str().to_string(),
        };
        let payload = serde_json::to_value(&evt).map_err(|e| AppError::internal(e.to_string()))?;
        self.outbox.save(&outbox_msg(store.id.0, StoreCreatedIntegrationEvent::TOPIC, payload)).await?;

        let _ = store.drain_events();
        info!(store_id = store.id.0, "Store created.");
        Ok(map_store(&store))
    }
}

register_command_handler!(CreateStore, AppDeps, |d: &AppDeps| {
    CreateStoreHandler { store_repo: d.store_repo.clone(), outbox: d.outbox.clone() }
});

// ── UpdateStoreInformationHandler ─────────────────────────────────────────────

pub struct UpdateStoreInformationHandler {
    store_repo: Arc<dyn StoreRepository>,
}

#[async_trait]
impl CommandHandler<UpdateStoreInformation> for UpdateStoreInformationHandler {
    async fn handle(&self, cmd: UpdateStoreInformation) -> AppResult<StoreDto> {
        let mut store = self.store_repo.find_by_id(cmd.store_id).await?
            .ok_or_else(|| AppError::not_found("Store", cmd.store_id.to_string()))?;
        store.update_information(
            cmd.name, cmd.address_street, cmd.address_city, cmd.address_postal_code,
            cmd.address_country, cmd.phone, cmd.geo_latitude, cmd.geo_longitude,
            cmd.schedules, cmd.services,
        )?;
        self.store_repo.save(&mut store).await?;
        let _ = store.drain_events();
        Ok(map_store(&store))
    }
}

register_command_handler!(UpdateStoreInformation, AppDeps, |d: &AppDeps| {
    UpdateStoreInformationHandler { store_repo: d.store_repo.clone() }
});

// ── ChangeStoreStatusHandler ──────────────────────────────────────────────────

pub struct ChangeStoreStatusHandler {
    store_repo: Arc<dyn StoreRepository>,
    outbox:     Arc<dyn OutboxRepository>,
}

#[async_trait]
impl CommandHandler<ChangeStoreStatus> for ChangeStoreStatusHandler {
    async fn handle(&self, cmd: ChangeStoreStatus) -> AppResult<()> {
        let mut store = self.store_repo.find_by_id(cmd.store_id).await?
            .ok_or_else(|| AppError::not_found("Store", cmd.store_id.to_string()))?;
        let old_status = store.status.as_str().to_string();
        store.change_status(cmd.status);
        let new_status = store.status.as_str().to_string();
        self.store_repo.save(&mut store).await?;

        let evt = StoreStatusChangedIntegrationEvent {
            store_id: store.id.0, old_status, new_status,
        };
        let payload = serde_json::to_value(&evt).map_err(|e| AppError::internal(e.to_string()))?;
        self.outbox.save(&outbox_msg(store.id.0, StoreStatusChangedIntegrationEvent::TOPIC, payload)).await?;
        let _ = store.drain_events();
        Ok(())
    }
}

register_command_handler!(ChangeStoreStatus, AppDeps, |d: &AppDeps| {
    ChangeStoreStatusHandler { store_repo: d.store_repo.clone(), outbox: d.outbox.clone() }
});

// ── UploadStoreLogoHandler ────────────────────────────────────────────────────

pub struct UploadStoreLogoHandler {
    store_repo:   Arc<dyn StoreRepository>,
    blob_storage: Arc<dyn BlobStorage>,
    blob_bucket:  String,
}

#[async_trait]
impl CommandHandler<UploadStoreLogo> for UploadStoreLogoHandler {
    async fn handle(&self, cmd: UploadStoreLogo) -> AppResult<String> {
        let mut store = self.store_repo.find_by_id(cmd.store_id).await?
            .ok_or_else(|| AppError::not_found("Store", cmd.store_id.to_string()))?;
        let ext = cmd.file_name.rsplit('.').next().unwrap_or("bin");
        let key = format!("stores/{}/logo.{}", store.id.0, ext);
        self.blob_storage.upload(&self.blob_bucket, &key, &cmd.content_type, cmd.file_content).await?;
        store.set_logo(key.clone());
        self.store_repo.save(&mut store).await?;
        let _ = store.drain_events();
        let url = self.blob_storage.presigned_get(&self.blob_bucket, &key, Duration::from_secs(3600)).await?.url;
        Ok(url)
    }
}

register_command_handler!(UploadStoreLogo, AppDeps, |d: &AppDeps| {
    UploadStoreLogoHandler {
        store_repo:   d.store_repo.clone(),
        blob_storage: d.blob_storage.clone(),
        blob_bucket:  d.blob_bucket.clone(),
    }
});

// ── DeleteStoreLogoHandler ────────────────────────────────────────────────────

pub struct DeleteStoreLogoHandler {
    store_repo:   Arc<dyn StoreRepository>,
    blob_storage: Arc<dyn BlobStorage>,
    blob_bucket:  String,
}

#[async_trait]
impl CommandHandler<DeleteStoreLogo> for DeleteStoreLogoHandler {
    async fn handle(&self, cmd: DeleteStoreLogo) -> AppResult<()> {
        let mut store = self.store_repo.find_by_id(cmd.store_id).await?
            .ok_or_else(|| AppError::not_found("Store", cmd.store_id.to_string()))?;
        if let Some(ref key) = store.logo_object_name.clone() {
            self.blob_storage.delete(&self.blob_bucket, key).await?;
        }
        store.remove_logo();
        self.store_repo.save(&mut store).await?;
        let _ = store.drain_events();
        Ok(())
    }
}

register_command_handler!(DeleteStoreLogo, AppDeps, |d: &AppDeps| {
    DeleteStoreLogoHandler {
        store_repo:   d.store_repo.clone(),
        blob_storage: d.blob_storage.clone(),
        blob_bucket:  d.blob_bucket.clone(),
    }
});

// ── Query handlers ────────────────────────────────────────────────────────────

pub struct GetStoreHandler { store_repo: Arc<dyn StoreRepository> }

#[async_trait]
impl QueryHandler<GetStore> for GetStoreHandler {
    async fn handle(&self, q: GetStore) -> AppResult<Option<StoreDto>> {
        Ok(self.store_repo.find_by_id(q.store_id).await?.map(|s| map_store(&s)))
    }
}

register_query_handler!(GetStore, AppDeps, |d: &AppDeps| {
    GetStoreHandler { store_repo: d.store_repo.clone() }
});

pub struct ListStoresHandler { store_repo: Arc<dyn StoreRepository> }

#[async_trait]
impl QueryHandler<ListStores> for ListStoresHandler {
    async fn handle(&self, q: ListStores) -> AppResult<PagedStoreDto> {
        let page      = if q.page < 1 { 1 } else { q.page };
        let page_size = if q.page_size < 1 { 20 } else { q.page_size };
        let result = self.store_repo.get_paged(
            page, page_size, q.search_term.as_deref(), q.status, q.location_id,
        ).await?;
        Ok(PagedStoreDto {
            items:     result.items.iter().map(map_store).collect(),
            total:     result.total,
            page:      result.page,
            page_size: result.page_size,
        })
    }
}

register_query_handler!(ListStores, AppDeps, |d: &AppDeps| {
    ListStoresHandler { store_repo: d.store_repo.clone() }
});

pub struct GetStoreLogoUrlHandler {
    store_repo:   Arc<dyn StoreRepository>,
    blob_storage: Arc<dyn BlobStorage>,
    blob_bucket:  String,
}

#[async_trait]
impl QueryHandler<GetStoreLogoUrl> for GetStoreLogoUrlHandler {
    async fn handle(&self, q: GetStoreLogoUrl) -> AppResult<Option<String>> {
        let store = self.store_repo.find_by_id(q.store_id).await?
            .ok_or_else(|| AppError::not_found("Store", q.store_id.to_string()))?;
        match store.logo_object_name {
            Some(ref key) => {
                let url = self.blob_storage.presigned_get(&self.blob_bucket, key, Duration::from_secs(3600)).await?.url;
                Ok(Some(url))
            }
            None => Ok(None),
        }
    }
}

register_query_handler!(GetStoreLogoUrl, AppDeps, |d: &AppDeps| {
    GetStoreLogoUrlHandler {
        store_repo:   d.store_repo.clone(),
        blob_storage: d.blob_storage.clone(),
        blob_bucket:  d.blob_bucket.clone(),
    }
});
