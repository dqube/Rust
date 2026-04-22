use chrono::{DateTime, Utc};
use ddd_shared_kernel::{AppError, AppResult};
use serde::{Deserialize, Serialize};

use super::ids::{CityCode, CountryCode, CurrencyCode, PincodeId, StateCode};

// ─── Currency ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Currency {
    pub id: CurrencyCode,
    pub name: String,
    pub symbol: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl Currency {
    pub fn create(code: CurrencyCode, name: String, symbol: String) -> AppResult<Self> {
        if code.0.len() != 3 || !code.0.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(AppError::validation(
                "code",
                "Currency code must be 3 uppercase letters (ISO 4217).",
            ));
        }
        if name.is_empty() || name.len() > 50 {
            return Err(AppError::validation(
                "name",
                "Currency name must be 1-50 characters.",
            ));
        }
        if symbol.is_empty() || symbol.len() > 5 {
            return Err(AppError::validation(
                "symbol",
                "Currency symbol must be 1-5 characters.",
            ));
        }
        Ok(Self {
            id: code,
            name,
            symbol,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
        })
    }

    pub fn update(&mut self, name: String, symbol: String) -> AppResult<()> {
        let has_name = !name.trim().is_empty();
        let has_symbol = !symbol.trim().is_empty();
        if !has_name && !has_symbol {
            return Err(AppError::validation(
                "name",
                "At least one of name or symbol must be provided.",
            ));
        }
        if has_name {
            if name.len() > 50 {
                return Err(AppError::validation(
                    "name",
                    "Currency name must be 1-50 characters.",
                ));
            }
            self.name = name;
        }
        if has_symbol {
            if symbol.len() > 5 {
                return Err(AppError::validation(
                    "symbol",
                    "Currency symbol must be 1-5 characters.",
                ));
            }
            self.symbol = symbol;
        }
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn activate(&mut self) {
        self.is_active = true;
        self.updated_at = Some(Utc::now());
    }
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.updated_at = Some(Utc::now());
    }
}

// ─── Country ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Country {
    pub id: CountryCode,
    pub name: String,
    pub currency_code: CurrencyCode,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl Country {
    pub fn create(
        code: CountryCode,
        name: String,
        currency_code: CurrencyCode,
    ) -> AppResult<Self> {
        if code.0.len() != 2 || !code.0.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(AppError::validation(
                "code",
                "Country code must be 2 uppercase letters (ISO 3166-1 alpha-2).",
            ));
        }
        if name.is_empty() || name.len() > 100 {
            return Err(AppError::validation(
                "name",
                "Country name must be 1-100 characters.",
            ));
        }
        Ok(Self {
            id: code,
            name,
            currency_code,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
        })
    }

    pub fn update(&mut self, name: String, currency_code: CurrencyCode) -> AppResult<()> {
        let has_name = !name.trim().is_empty();
        let has_currency = !currency_code.0.trim().is_empty();
        if !has_name && !has_currency {
            return Err(AppError::validation(
                "name",
                "At least one of name or currency code must be provided.",
            ));
        }
        if has_name {
            if name.len() > 100 {
                return Err(AppError::validation(
                    "name",
                    "Country name must be 1-100 characters.",
                ));
            }
            self.name = name;
        }
        if has_currency {
            if currency_code.0.len() != 3
                || !currency_code.0.chars().all(|c| c.is_ascii_alphabetic())
            {
                return Err(AppError::validation(
                    "currency_code",
                    "Currency code must be 3 uppercase letters (ISO 4217).",
                ));
            }
            self.currency_code = currency_code;
        }
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn activate(&mut self) {
        self.is_active = true;
        self.updated_at = Some(Utc::now());
    }
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.updated_at = Some(Utc::now());
    }
}

// ─── State ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub id: StateCode,
    pub name: String,
    pub country_code: CountryCode,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl State {
    pub fn create(
        code: StateCode,
        name: String,
        country_code: CountryCode,
    ) -> AppResult<Self> {
        if code.0.len() < 2 || code.0.len() > 10 {
            return Err(AppError::validation(
                "code",
                "State code must be 2-10 characters.",
            ));
        }
        if name.is_empty() || name.len() > 100 {
            return Err(AppError::validation(
                "name",
                "State name must be 1-100 characters.",
            ));
        }
        Ok(Self {
            id: code,
            name,
            country_code,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
        })
    }

    pub fn update(&mut self, name: String) -> AppResult<()> {
        if name.is_empty() || name.len() > 100 {
            return Err(AppError::validation(
                "name",
                "State name must be 1-100 characters.",
            ));
        }
        self.name = name;
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn activate(&mut self) {
        self.is_active = true;
        self.updated_at = Some(Utc::now());
    }
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.updated_at = Some(Utc::now());
    }
}

// ─── City ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct City {
    pub id: CityCode,
    pub name: String,
    pub state_code: StateCode,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl City {
    pub fn create(
        code: CityCode,
        name: String,
        state_code: StateCode,
    ) -> AppResult<Self> {
        if code.0.len() < 2 || code.0.len() > 20 {
            return Err(AppError::validation(
                "code",
                "City code must be 2-20 characters.",
            ));
        }
        if name.is_empty() || name.len() > 100 {
            return Err(AppError::validation(
                "name",
                "City name must be 1-100 characters.",
            ));
        }
        Ok(Self {
            id: code,
            name,
            state_code,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
        })
    }

    pub fn update(&mut self, name: String) -> AppResult<()> {
        if name.is_empty() || name.len() > 100 {
            return Err(AppError::validation(
                "name",
                "City name must be 1-100 characters.",
            ));
        }
        self.name = name;
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn activate(&mut self) {
        self.is_active = true;
        self.updated_at = Some(Utc::now());
    }
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.updated_at = Some(Utc::now());
    }
}

// ─── Pincode ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pincode {
    pub id: PincodeId,
    pub city_code: CityCode,
    pub area_name: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl Pincode {
    pub fn create(
        id: PincodeId,
        city_code: CityCode,
        area_name: Option<String>,
    ) -> AppResult<Self> {
        if id.0.len() < 3 || id.0.len() > 10 {
            return Err(AppError::validation(
                "code",
                "Pincode must be 3-10 characters.",
            ));
        }
        Ok(Self {
            id,
            city_code,
            area_name,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
        })
    }

    pub fn update(&mut self, area_name: Option<String>) {
        self.area_name = area_name;
        self.updated_at = Some(Utc::now());
    }

    pub fn activate(&mut self) {
        self.is_active = true;
        self.updated_at = Some(Utc::now());
    }
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.updated_at = Some(Utc::now());
    }
}
