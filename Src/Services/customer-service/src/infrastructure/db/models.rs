use sea_orm::entity::prelude::*;

pub mod customer {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "customers", schema_name = "customer")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        #[sea_orm(unique)]
        pub user_id: Uuid,
        pub first_name: String,
        pub last_name: String,
        pub email: Option<String>,
        #[sea_orm(unique)]
        pub membership_number: String,
        pub join_date: DateTimeWithTimeZone,
        pub expiry_date: DateTimeWithTimeZone,
        pub country_code: String,
        pub loyalty_points: i32,
        pub preferred_contact_method: Option<i32>,
        pub preferred_address_type: Option<i32>,
        pub created_at: DateTimeWithTimeZone,
        pub created_by: Option<String>,
        pub updated_at: Option<DateTimeWithTimeZone>,
        pub updated_by: Option<String>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod contact_number {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "customer_contact_numbers", schema_name = "customer")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub customer_id: Uuid,
        pub contact_type: i16,
        pub phone_number: String,
        pub is_primary: bool,
        pub verified: bool,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: Option<DateTimeWithTimeZone>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod address {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "customer_addresses", schema_name = "customer")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub customer_id: Uuid,
        pub label: String,
        pub address_type: i16,
        pub line1: String,
        pub line2: Option<String>,
        pub city: String,
        pub state: Option<String>,
        pub postal_code: String,
        pub country_code: String,
        pub is_primary: bool,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: Option<DateTimeWithTimeZone>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod profile {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "customer_profiles", schema_name = "customer")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        #[sea_orm(unique)]
        pub customer_id: Uuid,
        pub date_of_birth: Option<DateTimeWithTimeZone>,
        pub gender: Option<i16>,
        pub preferred_language: String,
        pub preferred_currency: String,
        pub tax_id: Option<String>,
        pub company_registration_number: Option<String>,
        pub email_notifications: bool,
        pub sms_notifications: bool,
        pub push_notifications: bool,
        pub marketing_emails: bool,
        pub kyc_status: String,
        pub kyc_verified_at: Option<DateTimeWithTimeZone>,
        #[sea_orm(column_type = "JsonBinary")]
        pub kyc_documents: Json,
        pub avatar_object_name: Option<String>,
        pub created_at: DateTimeWithTimeZone,
        pub created_by: Option<String>,
        pub updated_at: Option<DateTimeWithTimeZone>,
        pub updated_by: Option<String>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod wishlist_item {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "wishlist_items", schema_name = "customer")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub customer_id: Uuid,
        pub product_id: Uuid,
        pub product_name: String,
        #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
        pub base_price: Decimal,
        pub added_at: DateTimeWithTimeZone,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
