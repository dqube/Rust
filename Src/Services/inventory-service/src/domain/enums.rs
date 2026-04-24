use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementType {
    Purchase,
    Return,
    Adjustment,
    Damage,
    Transfer,
}

impl MovementType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Purchase => "Purchase",
            Self::Return => "Return",
            Self::Adjustment => "Adjustment",
            Self::Damage => "Damage",
            Self::Transfer => "Transfer",
        }
    }
}

impl FromStr for MovementType {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.eq_ignore_ascii_case("Purchase") {
            Ok(Self::Purchase)
        } else if value.eq_ignore_ascii_case("Return") {
            Ok(Self::Return)
        } else if value.eq_ignore_ascii_case("Adjustment") {
            Ok(Self::Adjustment)
        } else if value.eq_ignore_ascii_case("Damage") {
            Ok(Self::Damage)
        } else if value.eq_ignore_ascii_case("Transfer") {
            Ok(Self::Transfer)
        } else {
            Err(())
        }
    }
}