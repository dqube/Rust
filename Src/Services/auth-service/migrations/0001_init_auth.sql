-- Initial schema for auth-service: users, roles, role_permissions,
-- user_roles, refresh_tokens, password_reset_tokens.

CREATE SCHEMA IF NOT EXISTS auth;

CREATE TABLE IF NOT EXISTS auth.users (
    id                       UUID         PRIMARY KEY,
    username                 VARCHAR(100) NOT NULL UNIQUE,
    email                    VARCHAR(255) NOT NULL UNIQUE,
    email_confirmed          BOOLEAN      NOT NULL DEFAULT FALSE,
    phone_number             VARCHAR(30),
    phone_number_confirmed   BOOLEAN      NOT NULL DEFAULT FALSE,
    password_hash            TEXT         NOT NULL,
    security_stamp           VARCHAR(64)  NOT NULL,
    user_type                VARCHAR(20)  NOT NULL,
    two_factor_enabled       BOOLEAN      NOT NULL DEFAULT FALSE,
    two_factor_secret        VARCHAR(128),
    is_active                BOOLEAN      NOT NULL DEFAULT TRUE,
    is_locked                BOOLEAN      NOT NULL DEFAULT FALSE,
    lockout_end              TIMESTAMPTZ,
    failed_login_attempts    INTEGER      NOT NULL DEFAULT 0,
    created_at               TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ,
    last_login_at            TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_users_email    ON auth.users(LOWER(email));
CREATE INDEX IF NOT EXISTS idx_users_username ON auth.users(LOWER(username));

CREATE TABLE IF NOT EXISTS auth.roles (
    id          UUID         PRIMARY KEY,
    name        VARCHAR(100) NOT NULL UNIQUE,
    role_type   VARCHAR(20)  NOT NULL,
    description TEXT,
    is_active   BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS auth.role_permissions (
    role_id     UUID         NOT NULL REFERENCES auth.roles(id) ON DELETE CASCADE,
    permission  VARCHAR(150) NOT NULL,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_id, permission)
);
CREATE INDEX IF NOT EXISTS idx_role_permissions_permission ON auth.role_permissions(permission);

CREATE TABLE IF NOT EXISTS auth.user_roles (
    id          UUID         PRIMARY KEY,
    user_id     UUID         NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    role_id     UUID         NOT NULL REFERENCES auth.roles(id) ON DELETE CASCADE,
    assigned_by UUID,
    assigned_at TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ,
    UNIQUE (user_id, role_id)
);
CREATE INDEX IF NOT EXISTS idx_user_roles_user ON auth.user_roles(user_id);
CREATE INDEX IF NOT EXISTS idx_user_roles_role ON auth.user_roles(role_id);

CREATE TABLE IF NOT EXISTS auth.refresh_tokens (
    id           UUID         PRIMARY KEY,
    user_id      UUID         NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    token_hash   VARCHAR(128) NOT NULL UNIQUE,
    expires_at   TIMESTAMPTZ  NOT NULL,
    issued_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    revoked_at   TIMESTAMPTZ,
    replaced_by  UUID,
    ip_address   VARCHAR(64)
);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user ON auth.refresh_tokens(user_id);

CREATE TABLE IF NOT EXISTS auth.password_reset_tokens (
    id          UUID         PRIMARY KEY,
    user_id     UUID         NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    token_hash  VARCHAR(128) NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ  NOT NULL,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    used_at     TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_password_reset_tokens_user ON auth.password_reset_tokens(user_id);
