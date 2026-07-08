-- Create invoices table
CREATE TABLE IF NOT EXISTS invoices (
    id             BLOB PRIMARY KEY NOT NULL,
    client_id      BLOB NOT NULL REFERENCES clients(id) ON DELETE RESTRICT,
    invoice_number TEXT NOT NULL UNIQUE,
    status         TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft', 'sent', 'paid', 'void')),
    issued_date    TEXT NOT NULL,
    due_date       TEXT NOT NULL,
    total_amount   REAL NOT NULL DEFAULT 0.0,
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Add FK from time_entries to invoices (deferred to avoid circular migration)
CREATE INDEX IF NOT EXISTS idx_time_entries_invoice ON time_entries(invoice_id);
