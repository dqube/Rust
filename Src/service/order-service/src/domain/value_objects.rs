//! Order value objects.

use ddd_shared_kernel::impl_value_object;
use serde::{Deserialize, Serialize};

// ─── OrderStatus ─────────────────────────────────────────────────────────────

/// Lifecycle states for an order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Draft,
    Placed,
    Confirmed,
    Cancelled,
}

impl OrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Placed => "placed",
            Self::Confirmed => "confirmed",
            Self::Cancelled => "cancelled",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "draft" => Self::Draft,
            "placed" => Self::Placed,
            "confirmed" => Self::Confirmed,
            "cancelled" => Self::Cancelled,
            _ => Self::Draft,
        }
    }
}

impl std::fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl_value_object!(OrderStatus);

// ─── Money ───────────────────────────────────────────────────────────────────

/// A monetary amount (stored as cents to avoid floating-point issues).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    /// Amount in the smallest unit (cents).
    pub cents: i64,
}

impl Money {
    pub fn zero() -> Self {
        Self { cents: 0 }
    }

    pub fn from_cents(cents: i64) -> Self {
        Self { cents }
    }

    /// Create from a floating-point dollar amount (rounds to nearest cent).
    pub fn from_f64(amount: f64) -> Self {
        Self {
            cents: (amount * 100.0).round() as i64,
        }
    }

    /// Convert to f64 dollars.
    pub fn to_f64(&self) -> f64 {
        self.cents as f64 / 100.0
    }

    pub fn add(&self, other: &Money) -> Money {
        Money {
            cents: self.cents + other.cents,
        }
    }
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${:.2}", self.to_f64())
    }
}

impl_value_object!(Money);

// ─── OrderItem ───────────────────────────────────────────────────────────────

/// A line item within an order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderItem {
    pub sku: String,
    pub quantity: u32,
    pub unit_price: Money,
}

impl OrderItem {
    pub fn new(sku: impl Into<String>, quantity: u32, unit_price: Money) -> Self {
        Self {
            sku: sku.into(),
            quantity,
            unit_price,
        }
    }

    pub fn line_total(&self) -> Money {
        Money::from_cents(self.unit_price.cents * self.quantity as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_from_f64() {
        let m = Money::from_f64(19.99);
        assert_eq!(m.cents, 1999);
        assert!((m.to_f64() - 19.99).abs() < f64::EPSILON);
    }

    #[test]
    fn line_total() {
        let item = OrderItem::new("SKU-1", 3, Money::from_f64(10.00));
        assert_eq!(item.line_total().cents, 3000);
    }

    #[test]
    fn order_status_roundtrip() {
        assert_eq!(OrderStatus::from_str("confirmed"), OrderStatus::Confirmed);
        assert_eq!(OrderStatus::Confirmed.as_str(), "confirmed");
    }
}
