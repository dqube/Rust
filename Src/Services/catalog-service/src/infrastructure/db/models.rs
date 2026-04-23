use sea_orm::entity::prelude::*;

pub mod product {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "products", schema_name = "catalog")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:                            Uuid,
        pub sku:                           String,
        pub name:                          String,
        pub description:                   Option<String>,
        pub slug:                          Option<String>,
        pub category_id:                   i32,
        pub weight_grams:                  Option<i32>,
        pub width_cm:                      Option<i32>,
        pub height_cm:                     Option<i32>,
        pub depth_cm:                      Option<i32>,
        pub brand_id:                      Option<Uuid>,
        pub base_price:                    f64,
        pub cost_price:                    f64,
        pub is_taxable:                    bool,
        pub is_discontinued:               bool,
        pub discontinued_at:               Option<DateTimeWithTimeZone>,
        pub is_inventory_tracked:          bool,
        pub specifications:                Json,
        pub tags:                          Json,
        pub assigned_tax_config_ids:       Json,
        pub average_rating:                Option<f64>,
        pub total_reviews:                 i32,
        pub active_promotion_id:           Option<Uuid>,
        pub active_promotion_name:         Option<String>,
        pub promotion_discount_percentage: Option<f64>,
        pub promotion_valid_until:         Option<DateTimeWithTimeZone>,
        pub created_at:                    DateTimeWithTimeZone,
        pub created_by:                    Option<String>,
        pub updated_at:                    Option<DateTimeWithTimeZone>,
        pub updated_by:                    Option<String>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod product_variant {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "product_variants", schema_name = "catalog")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:                  Uuid,
        pub product_id:          Uuid,
        pub sku:                 String,
        pub description:         Option<String>,
        pub price_override:      Option<f64>,
        pub cost_price_override: Option<f64>,
        pub is_active:           bool,
        pub is_default:          bool,
        pub barcode:             Option<String>,
        pub barcode_type:        Option<String>,
        pub weight_grams:        Option<i32>,
        pub width_cm:            Option<i32>,
        pub height_cm:           Option<i32>,
        pub depth_cm:            Option<i32>,
        pub attributes:          Json,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod product_image {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "product_images", schema_name = "catalog")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:         Uuid,
        pub product_id: Uuid,
        pub url:        String,
        pub is_main:    bool,
        pub sort_order: i32,
        pub alt_text:   Option<String>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod country_pricing {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "country_pricing", schema_name = "catalog")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:             Uuid,
        pub product_id:     Uuid,
        pub country_code:   String,
        pub price:          f64,
        pub effective_date: DateTimeWithTimeZone,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod category {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "categories", schema_name = "catalog")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = true)]
        pub id:                 i32,
        pub name:               String,
        pub description:        Option<String>,
        pub slug:               Option<String>,
        pub parent_category_id: Option<i32>,
        pub image_url:          Option<String>,
        pub is_active:          bool,
        pub created_at:         DateTimeWithTimeZone,
        pub created_by:         Option<Uuid>,
        pub updated_at:         Option<DateTimeWithTimeZone>,
        pub updated_by:         Option<Uuid>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod brand {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "brands", schema_name = "catalog")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:          Uuid,
        pub name:        String,
        pub description: Option<String>,
        pub slug:        Option<String>,
        pub logo_url:    Option<String>,
        pub website:     Option<String>,
        pub is_active:   bool,
        pub created_at:  DateTimeWithTimeZone,
        pub created_by:  Option<String>,
        pub updated_at:  Option<DateTimeWithTimeZone>,
        pub updated_by:  Option<String>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod tax_configuration {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "tax_configurations", schema_name = "catalog")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:             Uuid,
        pub name:           String,
        pub code:           String,
        pub tax_type:       String,
        pub location_id:    i32,
        pub category_id:    Option<i32>,
        pub tax_rate:       f64,
        pub is_active:      bool,
        pub effective_date: DateTimeWithTimeZone,
        pub expiry_date:    Option<DateTimeWithTimeZone>,
        pub created_at:     DateTimeWithTimeZone,
        pub created_by:     Option<String>,
        pub updated_at:     Option<DateTimeWithTimeZone>,
        pub updated_by:     Option<String>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
