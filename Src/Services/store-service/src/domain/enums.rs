use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoreStatus {
    Active,
    Inactive,
    Maintenance,
    Closed,
}

impl StoreStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            StoreStatus::Active      => "Active",
            StoreStatus::Inactive    => "Inactive",
            StoreStatus::Maintenance => "Maintenance",
            StoreStatus::Closed      => "Closed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Active"      => StoreStatus::Active,
            "Inactive"    => StoreStatus::Inactive,
            "Maintenance" => StoreStatus::Maintenance,
            "Closed"      => StoreStatus::Closed,
            _             => StoreStatus::Inactive,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegisterStatus {
    Open,
    Closed,
}

impl RegisterStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RegisterStatus::Open   => "Open",
            RegisterStatus::Closed => "Closed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Open" => RegisterStatus::Open,
            _      => RegisterStatus::Closed,
        }
    }
}
