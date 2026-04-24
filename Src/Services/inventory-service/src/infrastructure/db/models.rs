use sea_orm::entity::prelude::*;

pub mod inventory_item {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "inventory_items", schema_name = "inventory")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub product_id: Uuid,
        pub product_variant_id: Option<Uuid>,
        pub store_id: i32,
        pub quantity: i32,
        pub reserved_quantity: i32,
        pub reorder_level: i32,
        pub last_restock_date: Option<DateTimeWithTimeZone>,
        pub created_at: DateTimeWithTimeZone,
        pub created_by: Option<String>,
        pub updated_at: DateTimeWithTimeZone,
        pub updated_by: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod stock_movement {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "stock_movements", schema_name = "inventory")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub product_id: Uuid,
        pub product_variant_id: Option<Uuid>,
        pub store_id: i32,
        pub quantity_change: i32,
        pub movement_type: String,
        pub movement_date: DateTimeWithTimeZone,
        pub employee_id: Option<Uuid>,
        pub reference_id: Option<String>,
        pub notes: Option<String>,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}