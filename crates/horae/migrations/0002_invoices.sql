-- Invoice tables for US3 (Phase 5).
-- Adds the invoice lifecycle: draft → sent → paid | void.

CREATE TYPE invoice_status AS ENUM ('draft', 'sent', 'paid', 'void');

CREATE TABLE invoices (
  id          uuid           PRIMARY KEY,
  org_id      uuid           NOT NULL REFERENCES organizations(id),
  client_id   uuid           NOT NULL REFERENCES clients(id),
  number      text           NOT NULL,
  status      invoice_status NOT NULL DEFAULT 'draft',
  issued_on   date           NOT NULL,
  due_on      date           NOT NULL,
  currency    char(3)        NOT NULL,
  total_cents bigint         NOT NULL DEFAULT 0,
  notes       text,
  created_at  timestamptz    NOT NULL DEFAULT now()
);

CREATE TABLE invoice_line_items (
  id            uuid    PRIMARY KEY,
  invoice_id    uuid    NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
  time_entry_id uuid    NOT NULL REFERENCES time_entries(id),
  description   text    NOT NULL,
  minutes       integer NOT NULL,
  rate_cents    bigint  NOT NULL,
  amount_cents  bigint  NOT NULL,
  UNIQUE (invoice_id, time_entry_id)
);

-- time_entries.invoice_id was added in 0001 as a bare uuid; add the FK now
-- that the invoices table exists.
ALTER TABLE time_entries
  ADD CONSTRAINT fk_time_entries_invoice
  FOREIGN KEY (invoice_id) REFERENCES invoices(id);
