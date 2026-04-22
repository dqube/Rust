-- Employee service schema bootstrap
CREATE SCHEMA IF NOT EXISTS employee;

-- Departments
CREATE TABLE IF NOT EXISTS employee.departments (
    id                      UUID PRIMARY KEY,
    department_name         VARCHAR(200) NOT NULL,
    department_code         VARCHAR(50) UNIQUE,
    parent_department_id    UUID,
    head_of_department_id   UUID,
    is_active               BOOLEAN NOT NULL DEFAULT TRUE,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_departments_code ON employee.departments(department_code);

-- Designations
CREATE TABLE IF NOT EXISTS employee.designations (
    id                  UUID PRIMARY KEY,
    designation_name    VARCHAR(200) NOT NULL UNIQUE,
    level               INTEGER,
    is_active           BOOLEAN NOT NULL DEFAULT TRUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_designations_name ON employee.designations(designation_name);

-- Employees
CREATE TABLE IF NOT EXISTS employee.employees (
    id                    UUID PRIMARY KEY,
    user_id               UUID NOT NULL UNIQUE,
    employee_code         VARCHAR(50) NOT NULL UNIQUE,
    first_name            VARCHAR(100) NOT NULL,
    last_name             VARCHAR(100) NOT NULL,
    middle_name           VARCHAR(100),
    date_of_birth         DATE,
    gender                VARCHAR(20),
    email                 VARCHAR(256) NOT NULL,
    personal_email        VARCHAR(256),
    phone                 VARCHAR(30),
    mobile                VARCHAR(30),
    department_id         UUID REFERENCES employee.departments(id),
    designation_id        UUID REFERENCES employee.designations(id),
    manager_id            UUID,
    employment_type       VARCHAR(30),
    date_of_joining       DATE NOT NULL,
    date_of_leaving       DATE,
    status                VARCHAR(30) NOT NULL DEFAULT 'Active',
    salary                NUMERIC(18, 2),
    bank_account_number   VARCHAR(50),
    bank_ifsc_code        VARCHAR(20),
    bank_name             VARCHAR(100),
    avatar_object_name    VARCHAR(500),
    current_store_id      INTEGER,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_employees_user_id    ON employee.employees(user_id);
CREATE INDEX IF NOT EXISTS idx_employees_code       ON employee.employees(employee_code);
CREATE INDEX IF NOT EXISTS idx_employees_email      ON employee.employees(email);
CREATE INDEX IF NOT EXISTS idx_employees_department ON employee.employees(department_id);
CREATE INDEX IF NOT EXISTS idx_employees_status     ON employee.employees(status);
