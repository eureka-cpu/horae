-- Create time_entries table
CREATE TABLE IF NOT EXISTS time_entries (
    id               BLOB PRIMARY KEY NOT NULL,
    user_id          BLOB NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    project_id       BLOB NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
    task_id          BLOB REFERENCES tasks(id) ON DELETE SET NULL,
    started_at       TEXT NOT NULL,
    ended_at         TEXT,  -- NULL means timer is still running
    duration_seconds INTEGER NOT NULL DEFAULT 0,
    notes            TEXT,
    is_billable      INTEGER NOT NULL DEFAULT 1,
    invoice_id       BLOB,  -- set when included in an invoice
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_time_entries_user ON time_entries(user_id);
CREATE INDEX IF NOT EXISTS idx_time_entries_project ON time_entries(project_id);
CREATE INDEX IF NOT EXISTS idx_time_entries_running ON time_entries(user_id) WHERE ended_at IS NULL;
