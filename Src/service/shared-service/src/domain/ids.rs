use serde::{Deserialize, Serialize};
use std::fmt;

/// ISO 4217 currency code — 3 uppercase letters, e.g. "MYR".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CurrencyCode(pub String);

impl CurrencyCode {
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into().to_uppercase())
    }
}

impl fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// ISO 3166-1 alpha-2 country code — 2 uppercase letters, e.g. "MY".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CountryCode(pub String);

impl CountryCode {
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into().to_uppercase())
    }
}

impl fmt::Display for CountryCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// State code — 2-10 chars, e.g. "SEL", "KUL".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StateCode(pub String);

impl StateCode {
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into().to_uppercase())
    }
}

impl fmt::Display for StateCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// City code — 2-20 chars, e.g. "KL", "PJ".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CityCode(pub String);

impl CityCode {
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into().to_uppercase())
    }
}

impl fmt::Display for CityCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Pincode ID — 3-10 chars, e.g. "50000", "50450".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PincodeId(pub String);

impl PincodeId {
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }
}

impl fmt::Display for PincodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
