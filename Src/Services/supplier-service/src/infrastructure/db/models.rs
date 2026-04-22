use sea_orm::entity::prelude::*;

pub mod supplier {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "suppliers", schema_name = "supplier")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:                        Uuid,
        pub user_id:                   Option<Uuid>,
        pub supplier_code:             String,
        pub company_name:              String,
        pub tax_identification_number: Option<String>,
        pub registration_number:       Option<String>,
        pub email:                     Option<String>,
        pub phone:                     Option<String>,
        pub website:                   Option<String>,
        pub business_type:             Option<String>,
        pub years_in_business:         Option<i32>,
        pub status:                    i32,
        pub onboarding_status:         i32,
        pub rating:                    Option<Decimal>,
        pub total_orders:              i32,
        pub notes:                     Option<String>,
        pub created_at:                DateTimeWithTimeZone,
        pub created_by:                Option<String>,
        pub updated_at:                Option<DateTimeWithTimeZone>,
        pub updated_by:                Option<String>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod supplier_address {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "supplier_addresses", schema_name = "supplier")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:           Uuid,
        pub supplier_id:  Uuid,
        pub address_type: i32,
        pub line1:        String,
        pub line2:        Option<String>,
        pub city:         String,
        pub state:        Option<String>,
        pub postal_code:  String,
        pub country:      String,
        pub is_primary:   bool,
        pub notes:        Option<String>,
        pub created_at:   DateTimeWithTimeZone,
        pub created_by:   Option<Uuid>,
        pub updated_at:   Option<DateTimeWithTimeZone>,
        pub updated_by:   Option<Uuid>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod supplier_contact {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "supplier_contacts", schema_name = "supplier")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:           Uuid,
        pub supplier_id:  Uuid,
        pub user_id:      Option<Uuid>,
        pub contact_type: i32,
        pub first_name:   String,
        pub last_name:    String,
        pub email:        Option<String>,
        pub phone:        Option<String>,
        pub mobile:       Option<String>,
        pub position:     Option<String>,
        pub department:   Option<String>,
        pub is_primary:   bool,
        pub can_login:    bool,
        pub notes:        Option<String>,
        pub created_at:   DateTimeWithTimeZone,
        pub created_by:   Option<String>,
        pub updated_at:   Option<DateTimeWithTimeZone>,
        pub updated_by:   Option<String>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod supplier_document {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "supplier_documents", schema_name = "supplier")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:            Uuid,
        pub supplier_id:   Uuid,
        pub file_name:     String,
        pub object_name:   String,
        pub content_type:  String,
        pub document_type: Option<String>,
        pub created_at:    DateTimeWithTimeZone,
        pub created_by:    Option<String>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod supplier_product {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "supplier_products", schema_name = "supplier")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:                 Uuid,
        pub supplier_id:        Uuid,
        pub product_id:         Uuid,
        pub variant_id:         Option<Uuid>,
        pub supplier_sku:       Option<String>,
        pub unit_cost:          Decimal,
        pub lead_time_days:     Option<i32>,
        pub min_order_quantity: Option<i32>,
        pub is_preferred:       bool,
        pub created_at:         DateTimeWithTimeZone,
        pub created_by:         Option<String>,
        pub updated_at:         Option<DateTimeWithTimeZone>,
        pub updated_by:         Option<String>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod purchase_order {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "purchase_orders", schema_name = "supplier")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:                  Uuid,
        pub supplier_id:         Uuid,
        pub store_id:            i32,
        pub order_date:          DateTimeWithTimeZone,
        pub expected_date:       Option<DateTimeWithTimeZone>,
        pub status:              String,
        pub total_amount:        Decimal,
        pub shipping_address_id: Option<Uuid>,
        pub contact_person_id:   Option<Uuid>,
        pub created_at:          DateTimeWithTimeZone,
        pub created_by:          Option<Uuid>,
        pub updated_at:          Option<DateTimeWithTimeZone>,
        pub updated_by:          Option<Uuid>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod purchase_order_detail {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "purchase_order_details", schema_name = "supplier")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:                Uuid,
        pub order_id:          Uuid,
        pub product_id:        Uuid,
        pub quantity:          i32,
        pub unit_cost:         Decimal,
        pub received_quantity: Option<i32>,
        pub created_at:        DateTimeWithTimeZone,
        pub created_by:        Option<Uuid>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
