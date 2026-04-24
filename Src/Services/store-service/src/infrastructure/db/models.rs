use sea_orm::entity::prelude::*;

pub mod store {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "stores", schema_name = "store")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id:                  i32,
        pub name:                String,
        pub location_id:         i32,
        pub address_street:      String,
        pub address_city:        String,
        pub address_postal_code: String,
        pub address_country:     String,
        pub phone:               String,
        pub geo_latitude:        f64,
        pub geo_longitude:       f64,
        pub schedules:           Json,
        pub services:            Json,
        pub status:              String,
        pub logo_object_name:    Option<String>,
        pub created_at:          DateTimeWithTimeZone,
        pub updated_at:          Option<DateTimeWithTimeZone>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod register {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "registers", schema_name = "store")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id:              i32,
        pub store_id:        i32,
        pub name:            String,
        pub current_balance: Decimal,
        pub status:          String,
        pub last_open:       Option<DateTimeWithTimeZone>,
        pub last_close:      Option<DateTimeWithTimeZone>,
        pub created_at:      DateTimeWithTimeZone,
        pub updated_at:      Option<DateTimeWithTimeZone>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
