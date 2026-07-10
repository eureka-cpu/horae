-- Horae Phase 1 schema
-- All IDs are UUID v7 (time-ordered).
-- Time stored as integer minutes; money as integer cents + ISO 4217 currency.

-- ── Enum types ───────────────────────────────────────────────────────────────

CREATE TYPE org_role     AS ENUM ('admin', 'manager', 'member');
CREATE TYPE project_role AS ENUM ('lead', 'freelancer', 'admin');
CREATE TYPE project_type AS ENUM ('time_and_materials', 'fixed_fee', 'non_billable', 'retainer');
CREATE TYPE entry_state  AS ENUM ('open', 'submitted', 'approved', 'invoiced');
CREATE TYPE budget_kind  AS ENUM ('none', 'amount', 'hours');
CREATE TYPE round_dir    AS ENUM ('nearest', 'up', 'down');

-- ── Tables ───────────────────────────────────────────────────────────────────

CREATE TABLE organizations (
  id               uuid        PRIMARY KEY,
  name             text        NOT NULL,
  default_currency char(3)     NOT NULL DEFAULT 'EUR',
  week_start       smallint    NOT NULL DEFAULT 1,       -- 1 = Monday
  round_minutes    smallint    NOT NULL DEFAULT 0,       -- 0 = no rounding
  round_dir        round_dir   NOT NULL DEFAULT 'nearest',
  created_at       timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE users (
  id                   uuid     PRIMARY KEY,
  org_id               uuid     NOT NULL REFERENCES organizations(id),
  email                text     NOT NULL UNIQUE,
  name                 text     NOT NULL,
  oidc_subject         text     UNIQUE,                  -- null until first OIDC login
  org_role             org_role NOT NULL DEFAULT 'member',
  cost_rate_cents      bigint,
  billable_rate_cents  bigint,
  active               boolean  NOT NULL DEFAULT true,
  created_at           timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE clients (
  id         uuid        PRIMARY KEY,
  org_id     uuid        NOT NULL REFERENCES organizations(id),
  name       text        NOT NULL,
  currency   char(3)     NOT NULL,
  address    text,
  tax_id     text,
  active     boolean     NOT NULL DEFAULT true,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE projects (
  id                  uuid         PRIMARY KEY,
  org_id              uuid         NOT NULL REFERENCES organizations(id),
  client_id           uuid         NOT NULL REFERENCES clients(id),
  code                text,
  name                text         NOT NULL,
  project_type        project_type NOT NULL DEFAULT 'time_and_materials',
  currency            char(3)      NOT NULL,
  starts_on           date,
  ends_on             date,
  budget_kind         budget_kind  NOT NULL DEFAULT 'none',
  budget_amount_cents bigint,
  budget_minutes      bigint,
  active              boolean      NOT NULL DEFAULT true,
  created_at          timestamptz  NOT NULL DEFAULT now()
);

CREATE TABLE tasks (                                      -- org-level task catalog
  id                 uuid    PRIMARY KEY,
  org_id             uuid    NOT NULL REFERENCES organizations(id),
  name               text    NOT NULL,
  billable_default   boolean NOT NULL DEFAULT true,
  default_rate_cents bigint,
  active             boolean NOT NULL DEFAULT true
);

CREATE TABLE project_tasks (                              -- task enabled per project + overrides
  project_id uuid    NOT NULL REFERENCES projects(id),
  task_id    uuid    NOT NULL REFERENCES tasks(id),
  billable   boolean NOT NULL,
  rate_cents bigint,
  PRIMARY KEY (project_id, task_id)
);

CREATE TABLE assignments (
  id         uuid         PRIMARY KEY,
  project_id uuid         NOT NULL REFERENCES projects(id),
  user_id    uuid         NOT NULL REFERENCES users(id),
  role       project_role NOT NULL DEFAULT 'freelancer',
  rate_cents bigint,
  created_at timestamptz  NOT NULL DEFAULT now(),
  UNIQUE (project_id, user_id)
);

CREATE TABLE time_entries (
  id              uuid        PRIMARY KEY,
  org_id          uuid        NOT NULL REFERENCES organizations(id),
  user_id         uuid        NOT NULL REFERENCES users(id),
  project_id      uuid        NOT NULL REFERENCES projects(id),
  task_id         uuid        NOT NULL REFERENCES tasks(id),
  spent_date      date        NOT NULL,
  minutes         integer     NOT NULL DEFAULT 0,
  rounded_minutes integer,                               -- persisted at lock (submit)
  notes           text,
  billable        boolean     NOT NULL,
  is_running      boolean     NOT NULL DEFAULT false,
  started_at      timestamptz,                           -- non-null only while running
  state           entry_state NOT NULL DEFAULT 'open',
  invoice_id      uuid,                                  -- Phase 4
  created_at      timestamptz NOT NULL DEFAULT now(),
  updated_at      timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX ON time_entries (user_id, spent_date);
CREATE INDEX ON time_entries (project_id, spent_date);

-- Enforces the one-running-timer-per-user invariant at the database level.
CREATE UNIQUE INDEX one_running_timer_per_user ON time_entries (user_id) WHERE is_running;

CREATE TABLE approvals (
  id           uuid        PRIMARY KEY,
  org_id       uuid        NOT NULL REFERENCES organizations(id),
  user_id      uuid        NOT NULL REFERENCES users(id),
  period_start date        NOT NULL,
  period_end   date        NOT NULL,
  state        entry_state NOT NULL DEFAULT 'submitted', -- reject deletes the row
  submitted_at timestamptz NOT NULL DEFAULT now(),
  approved_by  uuid        REFERENCES users(id),
  approved_at  timestamptz,
  UNIQUE (user_id, period_start)
);

CREATE TABLE audit_log (
  id            uuid        PRIMARY KEY,
  org_id        uuid        NOT NULL,
  actor_user_id uuid,
  action        text        NOT NULL,                    -- e.g. 'entry.submit'
  entity_type   text        NOT NULL,
  entity_id     uuid,
  data          jsonb,
  created_at    timestamptz NOT NULL DEFAULT now()
);
