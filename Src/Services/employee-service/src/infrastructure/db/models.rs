use sea_orm::entity::prelude::*;

pub mod employee {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "employees", schema_name = "employee")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:                    Uuid,
        pub user_id:               Uuid,
        pub employee_code:         String,
        pub first_name:            String,
        pub last_name:             String,
        pub middle_name:           Option<String>,
        pub date_of_birth:         Option<Date>,
        pub gender:                Option<String>,
        pub email:                 String,
        pub personal_email:        Option<String>,
        pub phone:                 Option<String>,
        pub mobile:                Option<String>,
        pub department_id:         Option<Uuid>,
        pub designation_id:        Option<Uuid>,
        pub manager_id:            Option<Uuid>,
        pub employment_type:       Option<String>,
        pub date_of_joining:       Date,
        pub date_of_leaving:       Option<Date>,
        pub status:                String,
        pub salary:                Option<Decimal>,
        pub bank_account_number:   Option<String>,
        pub bank_ifsc_code:        Option<String>,
        pub bank_name:             Option<String>,
        pub avatar_object_name:    Option<String>,
        pub current_store_id:      Option<i32>,
        pub created_at:            DateTimeWithTimeZone,
        pub updated_at:            Option<DateTimeWithTimeZone>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod department {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "departments", schema_name = "employee")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:                    Uuid,
        pub department_name:       String,
        pub department_code:       Option<String>,
        pub parent_department_id:  Option<Uuid>,
        pub head_of_department_id: Option<Uuid>,
        pub is_active:             bool,
        pub created_at:            DateTimeWithTimeZone,
        pub updated_at:            Option<DateTimeWithTimeZone>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod designation {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "designations", schema_name = "employee")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id:               Uuid,
        pub designation_name: String,
        pub level:            Option<i32>,
        pub is_active:        bool,
        pub created_at:       DateTimeWithTimeZone,
        pub updated_at:       Option<DateTimeWithTimeZone>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
