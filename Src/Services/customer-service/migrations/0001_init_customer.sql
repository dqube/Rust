-- Initial schema for customer-service: customers, contact numbers, addresses,
-- profiles (incl. KYC documents as JSONB), and wishlist items.

CREATE SCHEMA IF NOT EXISTS customer;

CREATE TABLE IF NOT EXISTS customer.customers (
    id                       UUID         PRIMARY KEY,
    user_id                  UUID         NOT NULL UNIQUE,
    first_name               VARCHAR(100) NOT NULL,
    last_name                VARCHAR(100) NOT NULL,
    email                    VARCHAR(255),
    membership_number        VARCHAR(40)  NOT NULL UNIQUE,
    join_date                TIMESTAMPTZ  NOT NULL,
    expiry_date              TIMESTAMPTZ  NOT NULL,
    country_code             VARCHAR(3)   NOT NULL,
    loyalty_points           INTEGER      NOT NULL DEFAULT 0,
    preferred_contact_method INTEGER,
    preferred_address_type   INTEGER,
    created_at               TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    created_by               VARCHAR(256),
    updated_at               TIMESTAMPTZ,
    updated_by               VARCHAR(256)
);
CREATE INDEX IF NOT EXISTS idx_customers_email             ON customer.customers(LOWER(email));
CREATE INDEX IF NOT EXISTS idx_customers_membership        ON customer.customers(membership_number);
CREATE INDEX IF NOT EXISTS idx_customers_country           ON customer.customers(country_code);

CREATE TABLE IF NOT EXISTS customer.customer_contact_numbers (
    id           UUID         PRIMARY KEY,
    customer_id  UUID         NOT NULL REFERENCES customer.customers(id) ON DELETE CASCADE,
    contact_type SMALLINT     NOT NULL DEFAULT 1,
    phone_number VARCHAR(30)  NOT NULL,
    is_primary   BOOLEAN      NOT NULL DEFAULT FALSE,
    verified     BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_contact_numbers_customer ON customer.customer_contact_numbers(customer_id);

CREATE TABLE IF NOT EXISTS customer.customer_addresses (
    id           UUID         PRIMARY KEY,
    customer_id  UUID         NOT NULL REFERENCES customer.customers(id) ON DELETE CASCADE,
    label        VARCHAR(100) NOT NULL,
    address_type SMALLINT     NOT NULL DEFAULT 3,
    line1        VARCHAR(200) NOT NULL,
    line2        VARCHAR(200),
    city         VARCHAR(100) NOT NULL,
    state        VARCHAR(100),
    postal_code  VARCHAR(20)  NOT NULL,
    country_code VARCHAR(3)   NOT NULL,
    is_primary   BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_addresses_customer ON customer.customer_addresses(customer_id);

CREATE TABLE IF NOT EXISTS customer.customer_profiles (
    id                           UUID         PRIMARY KEY,
    customer_id                  UUID         NOT NULL UNIQUE
                                               REFERENCES customer.customers(id) ON DELETE CASCADE,
    date_of_birth                TIMESTAMPTZ,
    gender                       SMALLINT,
    preferred_language           VARCHAR(10)  NOT NULL DEFAULT 'en',
    preferred_currency           VARCHAR(3)   NOT NULL DEFAULT 'USD',
    tax_id                       VARCHAR(50),
    company_registration_number  VARCHAR(100),
    email_notifications          BOOLEAN      NOT NULL DEFAULT TRUE,
    sms_notifications            BOOLEAN      NOT NULL DEFAULT TRUE,
    push_notifications           BOOLEAN      NOT NULL DEFAULT TRUE,
    marketing_emails             BOOLEAN      NOT NULL DEFAULT FALSE,
    kyc_status                   VARCHAR(30)  NOT NULL DEFAULT 'Pending',
    kyc_verified_at              TIMESTAMPTZ,
    kyc_documents                JSONB        NOT NULL DEFAULT '[]'::jsonb,
    avatar_object_name           VARCHAR(500),
    created_at                   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    created_by                   VARCHAR(256),
    updated_at                   TIMESTAMPTZ,
    updated_by                   VARCHAR(256)
);
CREATE INDEX IF NOT EXISTS idx_profiles_status ON customer.customer_profiles(kyc_status);

CREATE TABLE IF NOT EXISTS customer.wishlist_items (
    id           UUID           PRIMARY KEY,
    customer_id  UUID           NOT NULL REFERENCES customer.customers(id) ON DELETE CASCADE,
    product_id   UUID           NOT NULL,
    product_name VARCHAR(256)   NOT NULL,
    base_price   NUMERIC(18, 4) NOT NULL DEFAULT 0,
    added_at     TIMESTAMPTZ    NOT NULL DEFAULT NOW(),
    UNIQUE (customer_id, product_id)
);
CREATE INDEX IF NOT EXISTS idx_wishlist_customer ON customer.wishlist_items(customer_id);
