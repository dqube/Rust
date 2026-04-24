use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use ddd_domain::{define_aggregate, impl_aggregate, impl_aggregate_events};
use ddd_shared_kernel::{AppError, AppResult, DomainEvent};

use crate::domain::enums::RegisterStatus;
use crate::domain::events::{
    RegisterClosed, RegisterCreated, RegisterNameUpdated, RegisterOpened,
};
use crate::domain::ids::{RegisterId, StoreId};

define_aggregate!(Register, RegisterId, {
    pub store_id:        StoreId,
    pub name:            String,
    pub current_balance: Decimal,
    pub status:          RegisterStatus,
    pub last_open:       Option<DateTime<Utc>>,
    pub last_close:      Option<DateTime<Utc>>,
});

impl_aggregate!(Register, RegisterId);
impl_aggregate_events!(Register);

impl Register {
    pub fn create(store_id: StoreId, name: String) -> AppResult<Self> {
        Self::validate_name(&name)?;
        let now = Utc::now();
        Ok(Self {
            id: RegisterId(0),
            version: 0,
            created_at: now,
            updated_at: now,
            domain_events: Vec::new(),
            store_id,
            name,
            current_balance: Decimal::ZERO,
            status: RegisterStatus::Closed,
            last_open: None,
            last_close: None,
        })
    }

    /// Emit `RegisterCreated` after the repository has assigned the real id.
    pub fn emit_created(&mut self) {
        self.record_event(RegisterCreated {
            register_id: self.id,
            store_id:    self.store_id,
            name:        self.name.clone(),
            occurred_at: Utc::now(),
        });
    }

    pub fn open(&mut self, starting_cash: Decimal) -> AppResult<()> {
        if self.is_open() {
            return Err(AppError::conflict("Register is already open."));
        }
        if starting_cash.is_sign_negative() {
            return Err(AppError::validation("starting_cash", "must be non-negative"));
        }
        let now = Utc::now();
        self.status          = RegisterStatus::Open;
        self.current_balance = starting_cash;
        self.last_open       = Some(now);
        self.updated_at      = now;
        self.record_event(RegisterOpened {
            register_id:   self.id,
            store_id:      self.store_id,
            starting_cash,
            occurred_at:   now,
        });
        Ok(())
    }

    pub fn close(&mut self, ending_cash: Decimal) -> AppResult<Decimal> {
        if self.is_closed() {
            return Err(AppError::conflict("Register is already closed."));
        }
        if ending_cash.is_sign_negative() {
            return Err(AppError::validation("ending_cash", "must be non-negative"));
        }
        let variance        = ending_cash - self.current_balance;
        let now             = Utc::now();
        self.status         = RegisterStatus::Closed;
        self.current_balance = ending_cash;
        self.last_close     = Some(now);
        self.updated_at     = now;
        self.record_event(RegisterClosed {
            register_id: self.id,
            store_id:    self.store_id,
            ending_cash,
            variance,
            occurred_at: now,
        });
        Ok(variance)
    }

    pub fn add_cash(&mut self, amount: Decimal) -> AppResult<()> {
        if self.is_closed() {
            return Err(AppError::conflict("Cannot add cash to a closed register."));
        }
        if amount <= Decimal::ZERO {
            return Err(AppError::validation("amount", "must be positive"));
        }
        self.current_balance += amount;
        self.updated_at       = Utc::now();
        Ok(())
    }

    pub fn remove_cash(&mut self, amount: Decimal) -> AppResult<()> {
        if self.is_closed() {
            return Err(AppError::conflict("Cannot remove cash from a closed register."));
        }
        if amount <= Decimal::ZERO {
            return Err(AppError::validation("amount", "must be positive"));
        }
        if amount > self.current_balance {
            return Err(AppError::conflict(format!(
                "Insufficient balance: {} < {}",
                self.current_balance, amount,
            )));
        }
        self.current_balance -= amount;
        self.updated_at       = Utc::now();
        Ok(())
    }

    pub fn update_name(&mut self, name: String) -> AppResult<()> {
        Self::validate_name(&name)?;
        self.name       = name.clone();
        self.updated_at = Utc::now();
        self.record_event(RegisterNameUpdated {
            register_id: self.id,
            store_id:    self.store_id,
            new_name:    name,
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    pub fn is_open(&self)   -> bool { self.status == RegisterStatus::Open }
    pub fn is_closed(&self) -> bool { self.status == RegisterStatus::Closed }

    pub fn drain_events(&mut self) -> Vec<Box<dyn DomainEvent>> {
        std::mem::take(&mut self.domain_events)
    }

    fn validate_name(name: &str) -> AppResult<()> {
        if name.trim().is_empty() || name.len() > 50 {
            return Err(AppError::validation("name", "must be 1-50 characters"));
        }
        Ok(())
    }
}
