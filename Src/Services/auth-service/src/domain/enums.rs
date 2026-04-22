use ddd_shared_kernel::AppError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// User classification — drives default role assignment and business rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UserType {
    Customer,
    Employee,
    Supplier,
    Admin,
}

impl UserType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Customer => "Customer",
            Self::Employee => "Employee",
            Self::Supplier => "Supplier",
            Self::Admin => "Admin",
        }
    }
}

impl fmt::Display for UserType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for UserType {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Customer" | "customer" => Ok(Self::Customer),
            "Employee" | "employee" => Ok(Self::Employee),
            "Supplier" | "supplier" => Ok(Self::Supplier),
            "Admin" | "admin" => Ok(Self::Admin),
            other => Err(AppError::validation(
                "user_type",
                format!("unknown user type {other:?}"),
            )),
        }
    }
}

/// Role classification — built-in roles (seeded) versus custom roles created
/// through the admin API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleType {
    BuiltIn,
    Custom,
}

impl RoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BuiltIn => "BuiltIn",
            Self::Custom => "Custom",
        }
    }
}

impl fmt::Display for RoleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RoleType {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BuiltIn" | "builtin" | "built_in" => Ok(Self::BuiltIn),
            "Custom" | "custom" => Ok(Self::Custom),
            other => Err(AppError::validation(
                "role_type",
                format!("unknown role type {other:?}"),
            )),
        }
    }
}
