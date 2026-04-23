use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressType {
    Registered,
    Billing,
    Shipping,
    Warehouse,
}

impl AddressType {
    pub fn as_i32(&self) -> i32 {
        match self {
            AddressType::Registered => 1,
            AddressType::Billing    => 2,
            AddressType::Shipping   => 3,
            AddressType::Warehouse  => 4,
        }
    }
    pub fn from_i32(v: i32) -> Self {
        match v {
            2 => AddressType::Billing,
            3 => AddressType::Shipping,
            4 => AddressType::Warehouse,
            _ => AddressType::Registered,
        }
    }
}

impl FromStr for AddressType {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Billing"   => AddressType::Billing,
            "Shipping"  => AddressType::Shipping,
            "Warehouse" => AddressType::Warehouse,
            _           => AddressType::Registered,
        })
    }
}

impl fmt::Display for AddressType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AddressType::Registered => "Registered",
            AddressType::Billing    => "Billing",
            AddressType::Shipping   => "Shipping",
            AddressType::Warehouse  => "Warehouse",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContactType {
    Primary,
    Finance,
    Operations,
    Technical,
}

impl ContactType {
    pub fn as_i32(&self) -> i32 {
        match self {
            ContactType::Primary    => 1,
            ContactType::Finance    => 2,
            ContactType::Operations => 3,
            ContactType::Technical  => 4,
        }
    }
    pub fn from_i32(v: i32) -> Self {
        match v {
            2 => ContactType::Finance,
            3 => ContactType::Operations,
            4 => ContactType::Technical,
            _ => ContactType::Primary,
        }
    }
}

impl fmt::Display for ContactType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ContactType::Primary    => "Primary",
            ContactType::Finance    => "Finance",
            ContactType::Operations => "Operations",
            ContactType::Technical  => "Technical",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnboardingStatus {
    Pending,
    UnderReview,
    Approved,
    Rejected,
}

impl OnboardingStatus {
    pub fn as_i32(&self) -> i32 {
        match self {
            OnboardingStatus::Pending     => 1,
            OnboardingStatus::UnderReview => 2,
            OnboardingStatus::Approved    => 3,
            OnboardingStatus::Rejected    => 4,
        }
    }
    pub fn from_i32(v: i32) -> Self {
        match v {
            2 => OnboardingStatus::UnderReview,
            3 => OnboardingStatus::Approved,
            4 => OnboardingStatus::Rejected,
            _ => OnboardingStatus::Pending,
        }
    }
}

impl FromStr for OnboardingStatus {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "UnderReview" | "Under Review" => OnboardingStatus::UnderReview,
            "Approved"                     => OnboardingStatus::Approved,
            "Rejected"                     => OnboardingStatus::Rejected,
            _                              => OnboardingStatus::Pending,
        })
    }
}

impl fmt::Display for OnboardingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OnboardingStatus::Pending     => "Pending",
            OnboardingStatus::UnderReview => "UnderReview",
            OnboardingStatus::Approved    => "Approved",
            OnboardingStatus::Rejected    => "Rejected",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupplierStatus {
    Active,
    Inactive,
    Suspended,
    Pending,
}

impl SupplierStatus {
    pub fn as_i32(&self) -> i32 {
        match self {
            SupplierStatus::Active    => 1,
            SupplierStatus::Inactive  => 2,
            SupplierStatus::Suspended => 3,
            SupplierStatus::Pending   => 4,
        }
    }
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => SupplierStatus::Active,
            2 => SupplierStatus::Inactive,
            3 => SupplierStatus::Suspended,
            _ => SupplierStatus::Pending,
        }
    }
}

impl FromStr for SupplierStatus {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Active"    => SupplierStatus::Active,
            "Inactive"  => SupplierStatus::Inactive,
            "Suspended" => SupplierStatus::Suspended,
            _           => SupplierStatus::Pending,
        })
    }
}

impl fmt::Display for SupplierStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SupplierStatus::Active    => "Active",
            SupplierStatus::Inactive  => "Inactive",
            SupplierStatus::Suspended => "Suspended",
            SupplierStatus::Pending   => "Pending",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurchaseOrderStatus {
    Draft,
    Ordered,
    Received,
    Cancelled,
}

impl FromStr for PurchaseOrderStatus {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Ordered"   => PurchaseOrderStatus::Ordered,
            "Received"  => PurchaseOrderStatus::Received,
            "Cancelled" => PurchaseOrderStatus::Cancelled,
            _           => PurchaseOrderStatus::Draft,
        })
    }
}

impl fmt::Display for PurchaseOrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PurchaseOrderStatus::Draft     => "Draft",
            PurchaseOrderStatus::Ordered   => "Ordered",
            PurchaseOrderStatus::Received  => "Received",
            PurchaseOrderStatus::Cancelled => "Cancelled",
        };
        write!(f, "{s}")
    }
}
