use std::fmt;
use uuid::Uuid;

macro_rules! uuid_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        pub struct $name(pub Uuid);
        impl $name {
            pub fn new() -> Self { Self(Uuid::new_v4()) }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
        }
        impl Default for $name {
            fn default() -> Self { Self::new() }
        }
    };
}

uuid_id!(SaleId);
uuid_id!(SaleDetailId);
uuid_id!(AppliedDiscountId);
uuid_id!(ReturnId);
uuid_id!(ReturnDetailId);
