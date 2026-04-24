use serde::{Deserialize, Serialize};

// ── Published events ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreCreatedIntegrationEvent {
    pub store_id: i32,
    pub name:     String,
    pub city:     String,
    pub status:   String,
}

impl StoreCreatedIntegrationEvent {
    pub const TOPIC: &'static str = "v1.store.store.created";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreStatusChangedIntegrationEvent {
    pub store_id:   i32,
    pub old_status: String,
    pub new_status: String,
}

impl StoreStatusChangedIntegrationEvent {
    pub const TOPIC: &'static str = "v1.store.store.status-changed";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterCreatedIntegrationEvent {
    pub register_id: i32,
    pub store_id:    i32,
    pub name:        String,
}

impl RegisterCreatedIntegrationEvent {
    pub const TOPIC: &'static str = "v1.store.register.created";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterOpenedIntegrationEvent {
    pub register_id:   i32,
    pub store_id:      i32,
    pub starting_cash: String,
}

impl RegisterOpenedIntegrationEvent {
    pub const TOPIC: &'static str = "v1.store.register.opened";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterClosedIntegrationEvent {
    pub register_id: i32,
    pub store_id:    i32,
    pub ending_cash: String,
    pub variance:    String,
}

impl RegisterClosedIntegrationEvent {
    pub const TOPIC: &'static str = "v1.store.register.closed";
}

// ── Inbound events ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeStoreAssignedIntegrationEvent {
    pub employee_id:   uuid::Uuid,
    pub store_id:      i32,
    pub employee_code: String,
    pub first_name:    String,
    pub last_name:     String,
    pub assigned_at:   String,
}

impl EmployeeStoreAssignedIntegrationEvent {
    pub const TOPIC:   &'static str = "v1.employee.employee.store-assigned";
    pub const STREAM:  &'static str = "EMPLOYEE";
    pub const CONSUMER: &'static str = "store-service-employee-assigned";
}
