use sea_orm::entity::prelude::*;

pub mod sale {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "sales", schema_name = "sales")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:                     Uuid,
        pub store_id:               i32,
        pub employee_id:            Uuid,
        pub customer_id:            Option<Uuid>,
        pub register_id:            i32,
        pub receipt_number:         String,
        pub transaction_time:       DateTimeWithTimeZone,
        pub sub_total:              Decimal,
        pub discount_total:         Decimal,
        pub tax_amount:             Decimal,
        pub total_amount:           Decimal,
        pub channel:                String,
        pub status:                 String,
        pub shipping_address:       Option<Json>,
        pub billing_address:        Option<Json>,
        pub payment_transaction_id: Option<String>,
        pub receipt_object_name:    Option<String>,
        pub created_at:             DateTimeWithTimeZone,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod sale_detail {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "sale_details", schema_name = "sales")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:               Uuid,
        pub sale_id:          Uuid,
        pub product_id:       Uuid,
        pub variant_id:       Option<Uuid>,
        pub quantity:         i32,
        pub unit_price:       Decimal,
        pub applied_discount: Decimal,
        pub tax_applied:      Decimal,
        pub line_total:       Decimal,
        pub created_at:       DateTimeWithTimeZone,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod applied_discount {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "applied_discounts", schema_name = "sales")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:              Uuid,
        pub sale_id:         Uuid,
        pub sale_detail_id:  Option<Uuid>,
        pub campaign_id:     Uuid,
        pub rule_id:         Uuid,
        pub discount_amount: Decimal,
        pub created_at:      DateTimeWithTimeZone,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod return_entity {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "returns", schema_name = "sales")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:           Uuid,
        pub sale_id:      Uuid,
        pub return_date:  DateTimeWithTimeZone,
        pub employee_id:  Uuid,
        pub customer_id:  Option<Uuid>,
        pub total_refund: Decimal,
        pub created_at:   DateTimeWithTimeZone,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod return_detail {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "return_details", schema_name = "sales")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:         Uuid,
        pub return_id:  Uuid,
        pub product_id: Uuid,
        pub quantity:   i32,
        pub reason:     String,
        pub restock:    bool,
        pub created_at: DateTimeWithTimeZone,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod order_saga {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "order_sagas", schema_name = "sales")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub order_id:       Uuid,
        pub order_number:   String,
        pub customer_id:    Uuid,
        pub store_id:       i32,
        pub total:          Decimal,
        pub reservation_id: Option<Uuid>,
        pub payment_id:     Option<Uuid>,
        pub step:           String,
        pub failure_reason: Option<String>,
        pub items:          Json,   // Vec<SagaOrderItem>
        pub created_at:     DateTimeWithTimeZone,
        pub updated_at:     DateTimeWithTimeZone,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
