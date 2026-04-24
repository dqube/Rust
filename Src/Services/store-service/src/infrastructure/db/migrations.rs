use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_create_schema::Migration),
            Box::new(m20240102_create_stores::Migration),
            Box::new(m20240103_create_registers::Migration),
            Box::new(m20240104_create_outbox::Migration),
        ]
    }
}

mod m20240101_create_schema {
    use sea_orm_migration::prelude::*;
    pub struct Migration;
    impl MigrationName for Migration { fn name(&self) -> &str { "m20240101_create_schema" } }
    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
            m.get_connection().execute_unprepared("CREATE SCHEMA IF NOT EXISTS store;").await?;
            Ok(())
        }
        async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
            m.get_connection().execute_unprepared("DROP SCHEMA IF EXISTS store CASCADE;").await?;
            Ok(())
        }
    }
}

mod m20240102_create_stores {
    use sea_orm_migration::prelude::*;
    pub struct Migration;
    impl MigrationName for Migration { fn name(&self) -> &str { "m20240102_create_stores" } }
    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
            m.get_connection().execute_unprepared(r#"
                CREATE TABLE IF NOT EXISTS store.stores (
                    id                   SERIAL PRIMARY KEY,
                    name                 VARCHAR(100) NOT NULL UNIQUE,
                    location_id          INTEGER NOT NULL,
                    address_street       VARCHAR(200) NOT NULL,
                    address_city         VARCHAR(100) NOT NULL,
                    address_postal_code  VARCHAR(20) NOT NULL,
                    address_country      VARCHAR(100) NOT NULL,
                    phone                VARCHAR(20) NOT NULL,
                    geo_latitude         DOUBLE PRECISION NOT NULL DEFAULT 0,
                    geo_longitude        DOUBLE PRECISION NOT NULL DEFAULT 0,
                    schedules            JSONB NOT NULL DEFAULT '[]',
                    services             JSONB NOT NULL DEFAULT '[]',
                    status               VARCHAR(20) NOT NULL DEFAULT 'Active',
                    logo_object_name     VARCHAR(500),
                    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    updated_at           TIMESTAMPTZ
                );
                CREATE INDEX IF NOT EXISTS idx_stores_location ON store.stores(location_id);
                CREATE INDEX IF NOT EXISTS idx_stores_status   ON store.stores(status);
            "#).await?;
            Ok(())
        }
        async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
            m.get_connection().execute_unprepared("DROP TABLE IF EXISTS store.stores CASCADE;").await?;
            Ok(())
        }
    }
}

mod m20240103_create_registers {
    use sea_orm_migration::prelude::*;
    pub struct Migration;
    impl MigrationName for Migration { fn name(&self) -> &str { "m20240103_create_registers" } }
    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
            m.get_connection().execute_unprepared(r#"
                CREATE TABLE IF NOT EXISTS store.registers (
                    id               SERIAL PRIMARY KEY,
                    store_id         INTEGER NOT NULL REFERENCES store.stores(id) ON DELETE RESTRICT,
                    name             VARCHAR(50) NOT NULL,
                    current_balance  NUMERIC(19,4) NOT NULL DEFAULT 0,
                    status           VARCHAR(20) NOT NULL DEFAULT 'Closed',
                    last_open        TIMESTAMPTZ,
                    last_close       TIMESTAMPTZ,
                    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    updated_at       TIMESTAMPTZ,
                    UNIQUE (store_id, name)
                );
                CREATE INDEX IF NOT EXISTS idx_registers_store  ON store.registers(store_id);
                CREATE INDEX IF NOT EXISTS idx_registers_status ON store.registers(status);
            "#).await?;
            Ok(())
        }
        async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
            m.get_connection().execute_unprepared("DROP TABLE IF EXISTS store.registers CASCADE;").await?;
            Ok(())
        }
    }
}

mod m20240104_create_outbox {
    use sea_orm_migration::prelude::*;
    pub struct Migration;
    impl MigrationName for Migration { fn name(&self) -> &str { "m20240104_create_outbox" } }
    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
            m.get_connection().execute_unprepared(r#"
                CREATE TABLE IF NOT EXISTS store.outbox_messages (
                    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    aggregate_id   TEXT NOT NULL,
                    aggregate_type TEXT NOT NULL,
                    event_type     TEXT NOT NULL,
                    subject        TEXT NOT NULL,
                    payload        JSONB NOT NULL,
                    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    processed_at   TIMESTAMPTZ,
                    attempts       INT NOT NULL DEFAULT 0,
                    last_error     TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_store_outbox_pending
                    ON store.outbox_messages(created_at) WHERE processed_at IS NULL;
            "#).await?;
            Ok(())
        }
        async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
            m.get_connection().execute_unprepared("DROP TABLE IF EXISTS store.outbox_messages CASCADE;").await?;
            Ok(())
        }
    }
}
