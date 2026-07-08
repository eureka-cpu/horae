-- Create clients table
CREATE TABLE IF NOT EXISTS clients (
    id          BLOB PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL,
    email       TEXT,
    currency    TEXT NOT NULL DEFAULT 'USD',
    created_by  BLOB NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
