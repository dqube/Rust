use sea_orm::entity::prelude::*;

pub mod user {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "users", schema_name = "auth")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        #[sea_orm(unique)]
        pub username: String,
        #[sea_orm(unique)]
        pub email: String,
        pub email_confirmed: bool,
        pub phone_number: Option<String>,
        pub phone_number_confirmed: bool,
        pub password_hash: String,
        pub security_stamp: String,
        pub user_type: String,
        pub two_factor_enabled: bool,
        pub two_factor_secret: Option<String>,
        pub is_active: bool,
        pub is_locked: bool,
        pub lockout_end: Option<DateTimeWithTimeZone>,
        pub failed_login_attempts: i32,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: Option<DateTimeWithTimeZone>,
        pub last_login_at: Option<DateTimeWithTimeZone>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod role {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "roles", schema_name = "auth")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        #[sea_orm(unique)]
        pub name: String,
        pub role_type: String,
        pub description: Option<String>,
        pub is_active: bool,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: Option<DateTimeWithTimeZone>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod role_permission {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "role_permissions", schema_name = "auth")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub role_id: Uuid,
        #[sea_orm(primary_key, auto_increment = false)]
        pub permission: String,
        pub created_at: DateTimeWithTimeZone,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod user_role {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "user_roles", schema_name = "auth")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub user_id: Uuid,
        pub role_id: Uuid,
        pub assigned_by: Option<Uuid>,
        pub assigned_at: DateTimeWithTimeZone,
        pub expires_at: Option<DateTimeWithTimeZone>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod refresh_token {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "refresh_tokens", schema_name = "auth")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub user_id: Uuid,
        #[sea_orm(unique)]
        pub token_hash: String,
        pub expires_at: DateTimeWithTimeZone,
        pub issued_at: DateTimeWithTimeZone,
        pub revoked_at: Option<DateTimeWithTimeZone>,
        pub replaced_by: Option<Uuid>,
        pub ip_address: Option<String>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod password_reset_token {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "password_reset_tokens", schema_name = "auth")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub user_id: Uuid,
        #[sea_orm(unique)]
        pub token_hash: String,
        pub expires_at: DateTimeWithTimeZone,
        pub created_at: DateTimeWithTimeZone,
        pub used_at: Option<DateTimeWithTimeZone>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
