use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_create_schema::Migration),
            Box::new(m20240102_create_inventory_items::Migration),
            Box::new(m20240103_create_stock_movements::Migration),
        ]
    }
}

#[derive(DeriveIden)]
enum InventorySchema {
    #[sea_orm(iden = "inventory")]
    Schema,
}

#[derive(DeriveIden)]
enum InventoryItems {
    #[sea_orm(iden = "inventory_items")]
    Table,
    Id,
    ProductId,
    ProductVariantId,
    StoreId,
    Quantity,
    ReservedQuantity,
    ReorderLevel,
    LastRestockDate,
    CreatedAt,
    CreatedBy,
    UpdatedAt,
    UpdatedBy,
}

#[derive(DeriveIden)]
enum StockMovements {
    #[sea_orm(iden = "stock_movements")]
    Table,
    Id,
    ProductId,
    ProductVariantId,
    StoreId,
    QuantityChange,
    MovementType,
    MovementDate,
    EmployeeId,
    ReferenceId,
    Notes,
    CreatedAt,
    UpdatedAt,
}

#[path = "m20240101_create_schema.rs"]
mod m20240101_create_schema;

mod m20240102_create_inventory_items {
    use super::{InventoryItems, InventorySchema};
    use sea_orm::Statement;
    use sea_orm_migration::prelude::*;

    pub struct Migration;

    impl MigrationName for Migration {
        fn name(&self) -> &str {
            "m20240102_create_inventory_items"
        }
    }

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table((InventorySchema::Schema, InventoryItems::Table))
                        .if_not_exists()
                        .col(ColumnDef::new(InventoryItems::Id).uuid().not_null().primary_key())
                        .col(ColumnDef::new(InventoryItems::ProductId).uuid().not_null())
                        .col(ColumnDef::new(InventoryItems::ProductVariantId).uuid())
                        .col(ColumnDef::new(InventoryItems::StoreId).integer().not_null())
                        .col(ColumnDef::new(InventoryItems::Quantity).integer().not_null().default(0))
                        .col(ColumnDef::new(InventoryItems::ReservedQuantity).integer().not_null().default(0))
                        .col(ColumnDef::new(InventoryItems::ReorderLevel).integer().not_null().default(10))
                        .col(ColumnDef::new(InventoryItems::LastRestockDate).timestamp_with_time_zone())
                        .col(
                            ColumnDef::new(InventoryItems::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null()
                                .default(Expr::current_timestamp()),
                        )
                        .col(ColumnDef::new(InventoryItems::CreatedBy).string_len(255))
                        .col(
                            ColumnDef::new(InventoryItems::UpdatedAt)
                                .timestamp_with_time_zone()
                                .not_null()
                                .default(Expr::current_timestamp()),
                        )
                        .col(ColumnDef::new(InventoryItems::UpdatedBy).string_len(255))
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .name("idx_inventory_product")
                        .table((InventorySchema::Schema, InventoryItems::Table))
                        .col(InventoryItems::ProductId)
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .name("idx_inventory_store")
                        .table((InventorySchema::Schema, InventoryItems::Table))
                        .col(InventoryItems::StoreId)
                        .to_owned(),
                )
                .await?;

            manager
                .get_connection()
                .execute(Statement::from_string(
                    manager.get_database_backend(),
                    "CREATE UNIQUE INDEX IF NOT EXISTS idx_inventory_store_product ON inventory.inventory_items(store_id, product_id) WHERE product_variant_id IS NULL".to_owned(),
                ))
                .await?;
            manager
                .get_connection()
                .execute(Statement::from_string(
                    manager.get_database_backend(),
                    "CREATE UNIQUE INDEX IF NOT EXISTS idx_inventory_store_product_variant ON inventory.inventory_items(store_id, product_id, product_variant_id) WHERE product_variant_id IS NOT NULL".to_owned(),
                ))
                .await?;
            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(
                    Table::drop()
                        .table((InventorySchema::Schema, InventoryItems::Table))
                        .to_owned(),
                )
                .await
        }
    }
}

mod m20240103_create_stock_movements {
    use super::{InventorySchema, StockMovements};
    use sea_orm_migration::prelude::*;

    pub struct Migration;

    impl MigrationName for Migration {
        fn name(&self) -> &str {
            "m20240103_create_stock_movements"
        }
    }

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table((InventorySchema::Schema, StockMovements::Table))
                        .if_not_exists()
                        .col(ColumnDef::new(StockMovements::Id).uuid().not_null().primary_key())
                        .col(ColumnDef::new(StockMovements::ProductId).uuid().not_null())
                        .col(ColumnDef::new(StockMovements::ProductVariantId).uuid())
                        .col(ColumnDef::new(StockMovements::StoreId).integer().not_null())
                        .col(ColumnDef::new(StockMovements::QuantityChange).integer().not_null())
                        .col(ColumnDef::new(StockMovements::MovementType).string_len(20).not_null())
                        .col(
                            ColumnDef::new(StockMovements::MovementDate)
                                .timestamp_with_time_zone()
                                .not_null()
                                .default(Expr::current_timestamp()),
                        )
                        .col(ColumnDef::new(StockMovements::EmployeeId).uuid())
                        .col(ColumnDef::new(StockMovements::ReferenceId).string_len(100))
                        .col(ColumnDef::new(StockMovements::Notes).string_len(500))
                        .col(
                            ColumnDef::new(StockMovements::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null()
                                .default(Expr::current_timestamp()),
                        )
                        .col(
                            ColumnDef::new(StockMovements::UpdatedAt)
                                .timestamp_with_time_zone()
                                .not_null()
                                .default(Expr::current_timestamp()),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .name("idx_movements_date")
                        .table((InventorySchema::Schema, StockMovements::Table))
                        .col(StockMovements::MovementDate)
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .name("idx_movements_product")
                        .table((InventorySchema::Schema, StockMovements::Table))
                        .col(StockMovements::ProductId)
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .name("idx_movements_store")
                        .table((InventorySchema::Schema, StockMovements::Table))
                        .col(StockMovements::StoreId)
                        .to_owned(),
                )
                .await?;
            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(
                    Table::drop()
                        .table((InventorySchema::Schema, StockMovements::Table))
                        .to_owned(),
                )
                .await
        }
    }
}