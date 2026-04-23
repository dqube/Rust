-- Catalog service schema bootstrap
CREATE SCHEMA IF NOT EXISTS catalog;

CREATE TABLE IF NOT EXISTS catalog.categories (
    id                  SERIAL PRIMARY KEY,
    name                VARCHAR(256) NOT NULL,
    description         TEXT,
    slug                VARCHAR(256),
    parent_category_id  INTEGER REFERENCES catalog.categories(id) ON DELETE SET NULL,
    image_url           TEXT,
    is_active           BOOLEAN NOT NULL DEFAULT TRUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by          UUID,
    updated_at          TIMESTAMPTZ,
    updated_by          UUID
);
CREATE INDEX IF NOT EXISTS idx_categories_parent ON catalog.categories(parent_category_id);
CREATE INDEX IF NOT EXISTS idx_categories_slug   ON catalog.categories(slug);

CREATE TABLE IF NOT EXISTS catalog.brands (
    id          UUID PRIMARY KEY,
    name        VARCHAR(256) NOT NULL,
    description TEXT,
    slug        VARCHAR(256),
    logo_url    TEXT,
    website     VARCHAR(512),
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by  VARCHAR(256),
    updated_at  TIMESTAMPTZ,
    updated_by  VARCHAR(256)
);
CREATE INDEX IF NOT EXISTS idx_brands_name   ON catalog.brands(name);
CREATE INDEX IF NOT EXISTS idx_brands_active ON catalog.brands(is_active);

CREATE TABLE IF NOT EXISTS catalog.products (
    id                            UUID PRIMARY KEY,
    sku                           VARCHAR(100) NOT NULL UNIQUE,
    name                          VARCHAR(512) NOT NULL,
    description                   TEXT,
    slug                          VARCHAR(512),
    category_id                   INTEGER NOT NULL REFERENCES catalog.categories(id),
    weight_grams                  INTEGER,
    width_cm                      INTEGER,
    height_cm                     INTEGER,
    depth_cm                      INTEGER,
    brand_id                      UUID REFERENCES catalog.brands(id),
    base_price                    DOUBLE PRECISION NOT NULL DEFAULT 0,
    cost_price                    DOUBLE PRECISION NOT NULL DEFAULT 0,
    is_taxable                    BOOLEAN NOT NULL DEFAULT TRUE,
    is_discontinued               BOOLEAN NOT NULL DEFAULT FALSE,
    discontinued_at               TIMESTAMPTZ,
    is_inventory_tracked          BOOLEAN NOT NULL DEFAULT TRUE,
    specifications                JSONB NOT NULL DEFAULT '{}',
    tags                          JSONB NOT NULL DEFAULT '[]',
    assigned_tax_config_ids       JSONB NOT NULL DEFAULT '[]',
    average_rating                DOUBLE PRECISION,
    total_reviews                 INTEGER NOT NULL DEFAULT 0,
    active_promotion_id           UUID,
    active_promotion_name         VARCHAR(256),
    promotion_discount_percentage DOUBLE PRECISION,
    promotion_valid_until         TIMESTAMPTZ,
    created_at                    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by                    VARCHAR(256),
    updated_at                    TIMESTAMPTZ,
    updated_by                    VARCHAR(256)
);
CREATE INDEX IF NOT EXISTS idx_products_sku          ON catalog.products(sku);
CREATE INDEX IF NOT EXISTS idx_products_category     ON catalog.products(category_id);
CREATE INDEX IF NOT EXISTS idx_products_brand        ON catalog.products(brand_id);
CREATE INDEX IF NOT EXISTS idx_products_discontinued ON catalog.products(is_discontinued);

CREATE TABLE IF NOT EXISTS catalog.product_variants (
    id                  UUID PRIMARY KEY,
    product_id          UUID NOT NULL REFERENCES catalog.products(id) ON DELETE CASCADE,
    sku                 VARCHAR(100) NOT NULL,
    description         TEXT,
    price_override      DOUBLE PRECISION,
    cost_price_override DOUBLE PRECISION,
    is_active           BOOLEAN NOT NULL DEFAULT TRUE,
    is_default          BOOLEAN NOT NULL DEFAULT FALSE,
    barcode             VARCHAR(100),
    barcode_type        VARCHAR(50),
    weight_grams        INTEGER,
    width_cm            INTEGER,
    height_cm           INTEGER,
    depth_cm            INTEGER,
    attributes          JSONB NOT NULL DEFAULT '{}'
);
CREATE INDEX IF NOT EXISTS idx_variants_product ON catalog.product_variants(product_id);

CREATE TABLE IF NOT EXISTS catalog.product_images (
    id         UUID PRIMARY KEY,
    product_id UUID NOT NULL REFERENCES catalog.products(id) ON DELETE CASCADE,
    url        TEXT NOT NULL,
    is_main    BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    alt_text   VARCHAR(256)
);
CREATE INDEX IF NOT EXISTS idx_images_product ON catalog.product_images(product_id);

CREATE TABLE IF NOT EXISTS catalog.country_pricing (
    id              UUID PRIMARY KEY,
    product_id      UUID NOT NULL REFERENCES catalog.products(id) ON DELETE CASCADE,
    country_code    VARCHAR(3) NOT NULL,
    price           DOUBLE PRECISION NOT NULL,
    effective_date  TIMESTAMPTZ NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_pricing_product_country ON catalog.country_pricing(product_id, country_code);
CREATE INDEX IF NOT EXISTS idx_pricing_product ON catalog.country_pricing(product_id);

CREATE TABLE IF NOT EXISTS catalog.tax_configurations (
    id              UUID PRIMARY KEY,
    name            VARCHAR(256) NOT NULL,
    code            VARCHAR(50) NOT NULL UNIQUE,
    tax_type        VARCHAR(50) NOT NULL DEFAULT 'Percentage',
    location_id     INTEGER NOT NULL,
    category_id     INTEGER,
    tax_rate        DOUBLE PRECISION NOT NULL DEFAULT 0,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    effective_date  TIMESTAMPTZ NOT NULL,
    expiry_date     TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by      VARCHAR(256),
    updated_at      TIMESTAMPTZ,
    updated_by      VARCHAR(256)
);
CREATE INDEX IF NOT EXISTS idx_tax_location ON catalog.tax_configurations(location_id);
CREATE INDEX IF NOT EXISTS idx_tax_active   ON catalog.tax_configurations(is_active);
CREATE INDEX IF NOT EXISTS idx_tax_category ON catalog.tax_configurations(category_id);
