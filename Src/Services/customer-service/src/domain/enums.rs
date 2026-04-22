use ddd_shared_kernel::AppError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AddressType {
    Home,
    Work,
    Other,
}

impl AddressType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Work => "Work",
            Self::Other => "Other",
        }
    }
    pub fn from_i16(v: i16) -> Self {
        match v {
            2 => Self::Work,
            3 => Self::Other,
            _ => Self::Home,
        }
    }
    pub fn to_i16(self) -> i16 {
        match self {
            Self::Home => 1,
            Self::Work => 2,
            Self::Other => 3,
        }
    }
}

impl fmt::Display for AddressType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContactNumberType {
    Mobile,
    Home,
    Work,
    Other,
}

impl ContactNumberType {
    pub fn from_i16(v: i16) -> Self {
        match v {
            2 => Self::Home,
            3 => Self::Work,
            4 => Self::Other,
            _ => Self::Mobile,
        }
    }
    pub fn to_i16(self) -> i16 {
        match self {
            Self::Mobile => 1,
            Self::Home => 2,
            Self::Work => 3,
            Self::Other => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
    NonBinary,
    Other,
    PreferNotToSay,
}

impl Gender {
    pub fn from_i16(v: i16) -> Self {
        match v {
            2 => Self::Female,
            3 => Self::NonBinary,
            4 => Self::Other,
            5 => Self::PreferNotToSay,
            _ => Self::Male,
        }
    }
    pub fn to_i16(self) -> i16 {
        match self {
            Self::Male => 1,
            Self::Female => 2,
            Self::NonBinary => 3,
            Self::Other => 4,
            Self::PreferNotToSay => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KycStatus {
    Pending,
    Submitted,
    UnderReview,
    Verified,
    Rejected,
    Expired,
}

impl KycStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Submitted => "Submitted",
            Self::UnderReview => "UnderReview",
            Self::Verified => "Verified",
            Self::Rejected => "Rejected",
            Self::Expired => "Expired",
        }
    }
}

impl fmt::Display for KycStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for KycStatus {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Pending" | "pending" => Ok(Self::Pending),
            "Submitted" | "submitted" => Ok(Self::Submitted),
            "UnderReview" | "underreview" | "under_review" => Ok(Self::UnderReview),
            "Verified" | "verified" => Ok(Self::Verified),
            "Rejected" | "rejected" => Ok(Self::Rejected),
            "Expired" | "expired" => Ok(Self::Expired),
            other => Err(AppError::validation(
                "kyc_status",
                format!("unknown kyc status {other:?}"),
            )),
        }
    }
}
