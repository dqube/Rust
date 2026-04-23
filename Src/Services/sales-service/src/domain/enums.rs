use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending, Confirmed, Paid, Processing,
    Shipped, Delivered, Cancelled, Returned, Completed,
}

impl OrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderStatus::Pending    => "Pending",
            OrderStatus::Confirmed  => "Confirmed",
            OrderStatus::Paid       => "Paid",
            OrderStatus::Processing => "Processing",
            OrderStatus::Shipped    => "Shipped",
            OrderStatus::Delivered  => "Delivered",
            OrderStatus::Cancelled  => "Cancelled",
            OrderStatus::Returned   => "Returned",
            OrderStatus::Completed  => "Completed",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "Confirmed"  => OrderStatus::Confirmed,
            "Paid"       => OrderStatus::Paid,
            "Processing" => OrderStatus::Processing,
            "Shipped"    => OrderStatus::Shipped,
            "Delivered"  => OrderStatus::Delivered,
            "Cancelled"  => OrderStatus::Cancelled,
            "Returned"   => OrderStatus::Returned,
            "Completed"  => OrderStatus::Completed,
            _            => OrderStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SalesChannel {
    InStore, Online, MobileApp,
}

impl SalesChannel {
    pub fn as_str(&self) -> &'static str {
        match self {
            SalesChannel::InStore   => "InStore",
            SalesChannel::Online    => "Online",
            SalesChannel::MobileApp => "MobileApp",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "Online"    => SalesChannel::Online,
            "MobileApp" => SalesChannel::MobileApp,
            _           => SalesChannel::InStore,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReturnReason {
    Defective, WrongItem, CustomerChanged, Damaged, NotAsDescribed, Other,
}

impl ReturnReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReturnReason::Defective       => "Defective",
            ReturnReason::WrongItem       => "WrongItem",
            ReturnReason::CustomerChanged => "CustomerChanged",
            ReturnReason::Damaged         => "Damaged",
            ReturnReason::NotAsDescribed  => "NotAsDescribed",
            ReturnReason::Other           => "Other",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "WrongItem"       => ReturnReason::WrongItem,
            "CustomerChanged" => ReturnReason::CustomerChanged,
            "Damaged"         => ReturnReason::Damaged,
            "NotAsDescribed"  => ReturnReason::NotAsDescribed,
            "Other"           => ReturnReason::Other,
            _                 => ReturnReason::Defective,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSagaStep {
    WaitingForStockReservation,
    WaitingForPayment,
    Confirmed,
    Failed,
}

impl OrderSagaStep {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderSagaStep::WaitingForStockReservation => "WaitingForStockReservation",
            OrderSagaStep::WaitingForPayment          => "WaitingForPayment",
            OrderSagaStep::Confirmed                  => "Confirmed",
            OrderSagaStep::Failed                     => "Failed",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "WaitingForPayment" => OrderSagaStep::WaitingForPayment,
            "Confirmed"         => OrderSagaStep::Confirmed,
            "Failed"            => OrderSagaStep::Failed,
            _                   => OrderSagaStep::WaitingForStockReservation,
        }
    }
}
