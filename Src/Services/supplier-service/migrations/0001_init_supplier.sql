-- Supplier service schema bootstrap
CREATE SCHEMA IF NOT EXISTS supplier;

CREATE TABLE IF NOT EXISTS supplier.suppliers (
    id                        UUID PRIMARY KEY,
    user_id                   UUID,
    supplier_code             VARCHAR(50) NOT NULL UNIQUE,
    company_name              VARCHAR(300) NOT NULL,
    tax_identification_number VARCHAR(100),
    registration_number       VARCHAR(100),
    email                     VARCHAR(256),
    phone                     VARCHAR(50),
    website                   VARCHAR(500),
    business_type             VARCHAR(100),
    years_in_business         INTEGER,
    status                    INTEGER NOT NULL DEFAULT 4,  -- Pending=4
    onboarding_status         INTEGER NOT NULL DEFAULT 1,  -- Pending=1
    rating                    NUMERIC(4, 2),
    total_orders              INTEGER NOT NULL DEFAULT 0,
    notes                     TEXT,
    created_at                TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by                VARCHAR(256),
    updated_at                TIMESTAMPTZ,
    updated_by                VARCHAR(256)
);
CREATE INDEX IF NOT EXISTS idx_suppliers_code   ON supplier.suppliers(supplier_code);
CREATE INDEX IF NOT EXISTS idx_suppliers_status ON supplier.suppliers(status);
CREATE INDEX IF NOT EXISTS idx_suppliers_email  ON supplier.suppliers(email);

CREATE TABLE IF NOT EXISTS supplier.supplier_addresses (
    id           UUID PRIMARY KEY,
    supplier_id  UUID NOT NULL REFERENCES supplier.suppliers(id) ON DELETE CASCADE,
    address_type INTEGER NOT NULL DEFAULT 1,
    line1        VARCHAR(500) NOT NULL,
    line2        VARCHAR(500),
    city         VARCHAR(200) NOT NULL,
    state        VARCHAR(200),
    postal_code  VARCHAR(20) NOT NULL,
    country      VARCHAR(100) NOT NULL,
    is_primary   BOOLEAN NOT NULL DEFAULT FALSE,
    notes        TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by   UUID,
    updated_at   TIMESTAMPTZ,
    updated_by   UUID
);
CREATE INDEX IF NOT EXISTS idx_supplier_addresses_supplier ON supplier.supplier_addresses(supplier_id);

CREATE TABLE IF NOT EXISTS supplier.supplier_contacts (
    id           UUID PRIMARY KEY,
    supplier_id  UUID NOT NULL REFERENCES supplier.suppliers(id) ON DELETE CASCADE,
    user_id      UUID,
    contact_type INTEGER NOT NULL DEFAULT 1,
    first_name   VARCHAR(100) NOT NULL,
    last_name    VARCHAR(100) NOT NULL,
    email        VARCHAR(256),
    phone        VARCHAR(50),
    mobile       VARCHAR(50),
    position     VARCHAR(200),
    department   VARCHAR(200),
    is_primary   BOOLEAN NOT NULL DEFAULT FALSE,
    can_login    BOOLEAN NOT NULL DEFAULT FALSE,
    notes        TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by   VARCHAR(256),
    updated_at   TIMESTAMPTZ,
    updated_by   VARCHAR(256)
);
CREATE INDEX IF NOT EXISTS idx_supplier_contacts_supplier ON supplier.supplier_contacts(supplier_id);

CREATE TABLE IF NOT EXISTS supplier.supplier_documents (
    id            UUID PRIMARY KEY,
    supplier_id   UUID NOT NULL REFERENCES supplier.suppliers(id) ON DELETE CASCADE,
    file_name     VARCHAR(500) NOT NULL,
    object_name   VARCHAR(1000) NOT NULL,
    content_type  VARCHAR(200) NOT NULL,
    document_type VARCHAR(100),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by    VARCHAR(256)
);
CREATE INDEX IF NOT EXISTS idx_supplier_documents_supplier ON supplier.supplier_documents(supplier_id);

CREATE TABLE IF NOT EXISTS supplier.supplier_products (
    id                 UUID PRIMARY KEY,
    supplier_id        UUID NOT NULL REFERENCES supplier.suppliers(id) ON DELETE CASCADE,
    product_id         UUID NOT NULL,
    variant_id         UUID,
    supplier_sku       VARCHAR(100),
    unit_cost          NUMERIC(18, 4) NOT NULL,
    lead_time_days     INTEGER,
    min_order_quantity INTEGER,
    is_preferred       BOOLEAN NOT NULL DEFAULT FALSE,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by         VARCHAR(256),
    updated_at         TIMESTAMPTZ,
    updated_by         VARCHAR(256)
);
CREATE INDEX IF NOT EXISTS idx_supplier_products_supplier ON supplier.supplier_products(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_products_product  ON supplier.supplier_products(product_id);

CREATE TABLE IF NOT EXISTS supplier.purchase_orders (
    id                  UUID PRIMARY KEY,
    supplier_id         UUID NOT NULL REFERENCES supplier.suppliers(id),
    store_id            INTEGER NOT NULL,
    order_date          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expected_date       TIMESTAMPTZ,
    status              VARCHAR(50) NOT NULL DEFAULT 'Draft',
    total_amount        NUMERIC(18, 4) NOT NULL DEFAULT 0,
    shipping_address_id UUID,
    contact_person_id   UUID,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by          UUID,
    updated_at          TIMESTAMPTZ,
    updated_by          UUID
);
CREATE INDEX IF NOT EXISTS idx_purchase_orders_supplier ON supplier.purchase_orders(supplier_id);
CREATE INDEX IF NOT EXISTS idx_purchase_orders_status   ON supplier.purchase_orders(status);

CREATE TABLE IF NOT EXISTS supplier.purchase_order_details (
    id                UUID PRIMARY KEY,
    order_id          UUID NOT NULL REFERENCES supplier.purchase_orders(id) ON DELETE CASCADE,
    product_id        UUID NOT NULL,
    quantity          INTEGER NOT NULL,
    unit_cost         NUMERIC(18, 4) NOT NULL,
    received_quantity INTEGER,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by        UUID
);
CREATE INDEX IF NOT EXISTS idx_purchase_order_details_order ON supplier.purchase_order_details(order_id);
