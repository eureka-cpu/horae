-- Create projects table
CREATE TABLE IF NOT EXISTS projects (
    id              BLOB PRIMARY KEY NOT NULL,
    client_id       BLOB NOT NULL REFERENCES clients(id) ON DELETE RESTRICT,
    name            TEXT NOT NULL,
    code            TEXT,
    budget_hours    REAL,
    billing_method  TEXT NOT NULL DEFAULT 'hourly' CHECK(billing_method IN ('hourly', 'fixed', 'non_billable')),
    hourly_rate     REAL NOT NULL DEFAULT 0.0,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Create tasks table
CREATE TABLE IF NOT EXISTS tasks (
    id          BLOB PRIMARY KEY NOT NULL,
    project_id  BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    hourly_rate REAL NOT NULL DEFAULT 0.0,
    is_billable INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
