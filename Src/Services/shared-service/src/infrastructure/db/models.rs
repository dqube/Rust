use sea_orm::entity::prelude::*;

pub mod currency {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "currencies", schema_name = "shared")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub name: String,
        pub symbol: String,
        pub is_active: bool,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: Option<DateTimeWithTimeZone>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod country {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "countries", schema_name = "shared")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub name: String,
        pub currency_code: String,
        pub is_active: bool,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: Option<DateTimeWithTimeZone>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod state {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "states", schema_name = "shared")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub name: String,
        pub country_code: String,
        pub is_active: bool,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: Option<DateTimeWithTimeZone>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod city {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "cities", schema_name = "shared")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub name: String,
        pub state_code: String,
        pub is_active: bool,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: Option<DateTimeWithTimeZone>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod pincode {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "pincodes", schema_name = "shared")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub city_code: String,
        pub area_name: Option<String>,
        pub is_active: bool,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: Option<DateTimeWithTimeZone>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
