-- Sales service schema bootstrap
CREATE SCHEMA IF NOT EXISTS sales;

-- Sales (order headers)
CREATE TABLE IF NOT EXISTS sales.sales (
    id                      UUID PRIMARY KEY,
    store_id                INTEGER NOT NULL,
    employee_id             UUID NOT NULL,
    customer_id             UUID,
    register_id             INTEGER NOT NULL,
    receipt_number          VARCHAR(20) NOT NULL UNIQUE,
    transaction_time        TIMESTAMPTZ NOT NULL,
    sub_total               NUMERIC(18,2) NOT NULL DEFAULT 0,
    discount_total          NUMERIC(18,2) NOT NULL DEFAULT 0,
    tax_amount              NUMERIC(18,2) NOT NULL DEFAULT 0,
    total_amount            NUMERIC(18,2) NOT NULL DEFAULT 0,
    channel                 VARCHAR(20) NOT NULL DEFAULT 'InStore',
    status                  VARCHAR(20) NOT NULL DEFAULT 'Pending',
    shipping_address        JSONB,
    billing_address         JSONB,
    payment_transaction_id  VARCHAR(100),
    receipt_object_name     VARCHAR(500),
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_sales_store      ON sales.sales(store_id);
CREATE INDEX IF NOT EXISTS idx_sales_employee   ON sales.sales(employee_id);
CREATE INDEX IF NOT EXISTS idx_sales_customer   ON sales.sales(customer_id);
CREATE INDEX IF NOT EXISTS idx_sales_status     ON sales.sales(status);
CREATE INDEX IF NOT EXISTS idx_sales_created    ON sales.sales(created_at);

-- Sale details (line items)
CREATE TABLE IF NOT EXISTS sales.sale_details (
    id               UUID PRIMARY KEY,
    sale_id          UUID NOT NULL REFERENCES sales.sales(id) ON DELETE CASCADE,
    product_id       UUID NOT NULL,
    variant_id       UUID,
    quantity         INTEGER NOT NULL,
    unit_price       NUMERIC(18,2) NOT NULL,
    applied_discount NUMERIC(18,2) NOT NULL DEFAULT 0,
    tax_applied      NUMERIC(18,2) NOT NULL DEFAULT 0,
    line_total       NUMERIC(18,2) NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_sale_details_sale    ON sales.sale_details(sale_id);
CREATE INDEX IF NOT EXISTS idx_sale_details_product ON sales.sale_details(product_id);

-- Applied discounts
CREATE TABLE IF NOT EXISTS sales.applied_discounts (
    id              UUID PRIMARY KEY,
    sale_id         UUID NOT NULL REFERENCES sales.sales(id) ON DELETE CASCADE,
    sale_detail_id  UUID REFERENCES sales.sale_details(id) ON DELETE CASCADE,
    campaign_id     UUID NOT NULL,
    rule_id         UUID NOT NULL,
    discount_amount NUMERIC(18,2) NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_discounts_sale ON sales.applied_discounts(sale_id);

-- Returns
CREATE TABLE IF NOT EXISTS sales.returns (
    id           UUID PRIMARY KEY,
    sale_id      UUID NOT NULL REFERENCES sales.sales(id),
    return_date  TIMESTAMPTZ NOT NULL,
    employee_id  UUID NOT NULL,
    customer_id  UUID,
    total_refund NUMERIC(18,2) NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_returns_sale     ON sales.returns(sale_id);
CREATE INDEX IF NOT EXISTS idx_returns_employee ON sales.returns(employee_id);
CREATE INDEX IF NOT EXISTS idx_returns_customer ON sales.returns(customer_id);

-- Return details
CREATE TABLE IF NOT EXISTS sales.return_details (
    id         UUID PRIMARY KEY,
    return_id  UUID NOT NULL REFERENCES sales.returns(id) ON DELETE CASCADE,
    product_id UUID NOT NULL,
    quantity   INTEGER NOT NULL,
    reason     VARCHAR(30) NOT NULL,
    restock    BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_return_details_return ON sales.return_details(return_id);

-- Order sagas (orchestration state)
CREATE TABLE IF NOT EXISTS sales.order_sagas (
    order_id       UUID PRIMARY KEY,
    order_number   VARCHAR(50) NOT NULL,
    customer_id    UUID NOT NULL,
    store_id       INTEGER NOT NULL,
    total          NUMERIC(18,2) NOT NULL,
    reservation_id UUID,
    payment_id     UUID,
    step           VARCHAR(40) NOT NULL DEFAULT 'WaitingForStockReservation',
    failure_reason TEXT,
    items          JSONB NOT NULL DEFAULT '[]',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_order_sagas_step ON sales.order_sagas(step);

-- Outbox messages (ddd-infrastructure pattern: public schema)
CREATE TABLE IF NOT EXISTS outbox_messages (
    id              UUID PRIMARY KEY,
    aggregate_id    TEXT NOT NULL,
    aggregate_type  TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    subject         TEXT NOT NULL,
    payload         JSONB NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    published_at    TIMESTAMPTZ,
    attempts        INTEGER NOT NULL DEFAULT 0,
    last_error      TEXT
);
CREATE INDEX IF NOT EXISTS idx_outbox_unpublished ON outbox_messages(created_at) WHERE published_at IS NULL;
