use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::any::Any;

use ddd_shared_kernel::DomainEvent;

use crate::domain::enums::StoreStatus;
use crate::domain::ids::{RegisterId, StoreId};

macro_rules! domain_event {
    ($ty:ident, $name:literal) => {
        impl DomainEvent for $ty {
            fn event_name(&self) -> &'static str { $name }
            fn occurred_at(&self) -> DateTime<Utc> { self.occurred_at }
            fn as_any(&self) -> &dyn Any { self }
        }
    };
}

// ── Store lifecycle ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreCreated {
    pub store_id:    StoreId,
    pub name:        String,
    pub city:        String,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(StoreCreated, "store.store.created");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreInformationUpdated {
    pub store_id:    StoreId,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(StoreInformationUpdated, "store.store.information_updated");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreStatusChanged {
    pub store_id:    StoreId,
    pub old_status:  StoreStatus,
    pub new_status:  StoreStatus,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(StoreStatusChanged, "store.store.status_changed");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreLogoUpdated {
    pub store_id:    StoreId,
    pub object_name: Option<String>,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(StoreLogoUpdated, "store.store.logo_updated");

// ── Register lifecycle ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterCreated {
    pub register_id: RegisterId,
    pub store_id:    StoreId,
    pub name:        String,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(RegisterCreated, "store.register.created");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterOpened {
    pub register_id:   RegisterId,
    pub store_id:      StoreId,
    pub starting_cash: Decimal,
    pub occurred_at:   DateTime<Utc>,
}
domain_event!(RegisterOpened, "store.register.opened");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterClosed {
    pub register_id: RegisterId,
    pub store_id:    StoreId,
    pub ending_cash: Decimal,
    pub variance:    Decimal,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(RegisterClosed, "store.register.closed");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterNameUpdated {
    pub register_id: RegisterId,
    pub store_id:    StoreId,
    pub new_name:    String,
    pub occurred_at: DateTime<Utc>,
}
domain_event!(RegisterNameUpdated, "store.register.name_updated");
