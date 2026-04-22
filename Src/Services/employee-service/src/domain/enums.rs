use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmployeeStatus {
    Active,
    Inactive,
    OnLeave,
    Terminated,
}

impl fmt::Display for EmployeeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmployeeStatus::Active     => write!(f, "Active"),
            EmployeeStatus::Inactive   => write!(f, "Inactive"),
            EmployeeStatus::OnLeave    => write!(f, "OnLeave"),
            EmployeeStatus::Terminated => write!(f, "Terminated"),
        }
    }
}

impl FromStr for EmployeeStatus {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Active"     => Ok(EmployeeStatus::Active),
            "Inactive"   => Ok(EmployeeStatus::Inactive),
            "OnLeave"    => Ok(EmployeeStatus::OnLeave),
            "Terminated" => Ok(EmployeeStatus::Terminated),
            _            => Ok(EmployeeStatus::Active),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmploymentType {
    FullTime,
    PartTime,
    Contract,
    Intern,
}

impl fmt::Display for EmploymentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmploymentType::FullTime => write!(f, "FullTime"),
            EmploymentType::PartTime => write!(f, "PartTime"),
            EmploymentType::Contract => write!(f, "Contract"),
            EmploymentType::Intern   => write!(f, "Intern"),
        }
    }
}

impl FromStr for EmploymentType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PartTime" => Ok(EmploymentType::PartTime),
            "Contract" => Ok(EmploymentType::Contract),
            "Intern"   => Ok(EmploymentType::Intern),
            _          => Ok(EmploymentType::FullTime),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gender {
    Male,
    Female,
    Other,
}

impl fmt::Display for Gender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Gender::Male   => write!(f, "Male"),
            Gender::Female => write!(f, "Female"),
            Gender::Other  => write!(f, "Other"),
        }
    }
}

impl FromStr for Gender {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Female" => Ok(Gender::Female),
            "Other"  => Ok(Gender::Other),
            _        => Ok(Gender::Male),
        }
    }
}
