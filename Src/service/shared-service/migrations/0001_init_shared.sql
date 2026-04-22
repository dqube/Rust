-- Initial schema for shared-service: reference data tables
-- (currencies, countries, states, cities, pincodes).

CREATE SCHEMA IF NOT EXISTS shared;

CREATE TABLE IF NOT EXISTS shared.currencies (
    id         VARCHAR(3)  PRIMARY KEY,
    name       VARCHAR(50) NOT NULL,
    symbol     VARCHAR(5)  NOT NULL,
    is_active  BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS shared.countries (
    id            VARCHAR(2)   PRIMARY KEY,
    name          VARCHAR(100) NOT NULL,
    currency_code VARCHAR(3)   NOT NULL REFERENCES shared.currencies(id),
    is_active     BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_countries_currency ON shared.countries(currency_code);

CREATE TABLE IF NOT EXISTS shared.states (
    id           VARCHAR(10)  PRIMARY KEY,
    name         VARCHAR(100) NOT NULL,
    country_code VARCHAR(2)   NOT NULL REFERENCES shared.countries(id),
    is_active    BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_states_country ON shared.states(country_code);

CREATE TABLE IF NOT EXISTS shared.cities (
    id         VARCHAR(20)  PRIMARY KEY,
    name       VARCHAR(100) NOT NULL,
    state_code VARCHAR(10)  NOT NULL REFERENCES shared.states(id),
    is_active  BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_cities_state ON shared.cities(state_code);

CREATE TABLE IF NOT EXISTS shared.pincodes (
    id         VARCHAR(10)  PRIMARY KEY,
    city_code  VARCHAR(20)  NOT NULL REFERENCES shared.cities(id),
    area_name  VARCHAR(100),
    is_active  BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_pincodes_city ON shared.pincodes(city_code);
