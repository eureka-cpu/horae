# Phase 1 Build Spec — Harvest-Equivalent Tracker

Executable implementation spec for **Phase 1 only** (see `harvest-clone-requirements.md` §2 for
scope, §3 for the UI reference). This doc is concrete enough for a coding agent to build against
milestone by milestone. Where the requirements left choices open, this spec **pins a default**
(marked ⚙️) so implementation isn't blocked; defaults are changeable but shouldn't be re-litigated
mid-build.

**How to build with this:** implement milestones M0→M9 in order. Each milestone lists deliverables
and a **Done-when** checklist. Write tests as you go; a milestone isn't done until its tests pass.

---

## 0. Pinned decisions (⚙️ Phase-1 defaults)

- **Architecture:** ⚙️ **Axum REST backend + separate Dioxus web SPA** (not Dioxus fullstack
  server functions). Cleaner boundary, easier to test, and the REST API is reused by the future
  export API and desktop/mobile clients. JSON over HTTP; cookie session.
- **Language/stack:** Rust (edition 2021+). Backend: `axum`, `tokio`, `sqlx` (Postgres, compile-
  time-checked queries), `serde`. Frontend: `dioxus` (web target), `dioxus-router`. Auth:
  `openidconnect`. PDF: ⚙️ `typst` (deterministic, good typography) — fallback `printpdf`. Excel:
  `rust_xlsxwriter`. CSV: `csv`.
- **DB:** Postgres 15+. Migrations via `sqlx migrate`. ⚙️ Postgres-only for Phase 1 (no SQLite).
- **Time storage:** store durations as **integer minutes** (never floats) to keep totals exact.
- **IDs:** `uuid` v7 (time-ordered) primary keys.
- **Money:** store as integer minor units (cents) + ISO currency code; never float.
- **Auth for dev:** an OIDC provider is required in prod; for local dev provide a `DEV_LOGIN=1`
  bypass that logs in a seeded admin.
- **Single org:** one `organizations` row; everything is scoped to it but keep `org_id` FKs so
  multi-org is a later flip.

---

## 1. Repo / workspace layout

```
harvestclone/
  Cargo.toml                # workspace
  crates/
    core/                   # pure domain: types, rounding, state machine, totals. NO I/O.
    server/                 # axum app, sqlx, auth, handlers, exports
    web/                    # dioxus web SPA
  migrations/               # sqlx migrations (0001_init.sql, ...)
  templates/invoice/…       # (Phase 4) not needed now
  templates/timesheet.typ   # typst template for PDF timesheet export
  docker-compose.yml        # postgres + server + web
  Dockerfile.server
  Dockerfile.web
  .env.example
  README.md
```

`core` must not depend on `sqlx`/`axum`/`dioxus` — only `serde`, `time`/`chrono`, `rust_decimal`
if needed. Everything correctness-critical (rounding, totals, state transitions, duration parsing)
lives here and is unit-tested in isolation.

---

## 2. Database schema (migration `0001_init.sql`)

Enums:

```sql
CREATE TYPE org_role     AS ENUM ('admin','manager','member');
CREATE TYPE project_role AS ENUM ('lead','freelancer','admin');      -- per-assignment role
CREATE TYPE project_type AS ENUM ('time_and_materials','fixed_fee','non_billable','retainer');
CREATE TYPE entry_state  AS ENUM ('open','submitted','approved','invoiced');
CREATE TYPE budget_kind  AS ENUM ('none','amount','hours');
CREATE TYPE round_dir    AS ENUM ('nearest','up','down');
```

Tables:

```sql
CREATE TABLE organizations (
  id            uuid PRIMARY KEY,
  name          text NOT NULL,
  default_currency char(3) NOT NULL DEFAULT 'EUR',
  week_start    smallint NOT NULL DEFAULT 1,          -- 1 = Monday
  round_minutes smallint NOT NULL DEFAULT 0,          -- 0 = no rounding
  round_dir     round_dir NOT NULL DEFAULT 'nearest',
  created_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE users (
  id            uuid PRIMARY KEY,
  org_id        uuid NOT NULL REFERENCES organizations(id),
  email         text NOT NULL UNIQUE,
  name          text NOT NULL,
  oidc_subject  text UNIQUE,                           -- null until first OIDC login
  org_role      org_role NOT NULL DEFAULT 'member',
  cost_rate_cents     bigint,                          -- optional
  billable_rate_cents bigint,
  active        boolean NOT NULL DEFAULT true,
  created_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE clients (
  id            uuid PRIMARY KEY,
  org_id        uuid NOT NULL REFERENCES organizations(id),
  name          text NOT NULL,
  currency      char(3) NOT NULL,                      -- default currency
  address       text,
  tax_id        text,
  active        boolean NOT NULL DEFAULT true,
  created_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE projects (
  id            uuid PRIMARY KEY,
  org_id        uuid NOT NULL REFERENCES organizations(id),
  client_id     uuid NOT NULL REFERENCES clients(id),
  code          text,                                  -- e.g. '7307'
  name          text NOT NULL,
  project_type  project_type NOT NULL DEFAULT 'time_and_materials',
  currency      char(3) NOT NULL,                      -- defaults from client at create time
  starts_on     date,
  ends_on       date,
  budget_kind   budget_kind NOT NULL DEFAULT 'none',
  budget_amount_cents bigint,                          -- if budget_kind='amount'
  budget_minutes bigint,                               -- if budget_kind='hours'
  active        boolean NOT NULL DEFAULT true,
  created_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE tasks (                                    -- global catalog
  id            uuid PRIMARY KEY,
  org_id        uuid NOT NULL REFERENCES organizations(id),
  name          text NOT NULL,
  billable_default boolean NOT NULL DEFAULT true,
  default_rate_cents bigint,
  active        boolean NOT NULL DEFAULT true
);

CREATE TABLE project_tasks (                            -- enable + override per project
  project_id    uuid NOT NULL REFERENCES projects(id),
  task_id       uuid NOT NULL REFERENCES tasks(id),
  billable      boolean NOT NULL,
  rate_cents    bigint,
  PRIMARY KEY (project_id, task_id)
);

CREATE TABLE assignments (
  id            uuid PRIMARY KEY,
  project_id    uuid NOT NULL REFERENCES projects(id),
  user_id       uuid NOT NULL REFERENCES users(id),
  role          project_role NOT NULL DEFAULT 'freelancer',
  rate_cents    bigint,                                 -- overrides user default
  created_at    timestamptz NOT NULL DEFAULT now(),
  UNIQUE (project_id, user_id)
);

CREATE TABLE time_entries (
  id            uuid PRIMARY KEY,
  org_id        uuid NOT NULL REFERENCES organizations(id),
  user_id       uuid NOT NULL REFERENCES users(id),
  project_id    uuid NOT NULL REFERENCES projects(id),
  task_id       uuid NOT NULL REFERENCES tasks(id),
  spent_date    date NOT NULL,
  minutes       integer NOT NULL DEFAULT 0,            -- precise tracked minutes
  rounded_minutes integer,                             -- persisted at lock (submit)
  notes         text,
  billable      boolean NOT NULL,
  is_running    boolean NOT NULL DEFAULT false,
  started_at    timestamptz,                            -- when running
  state         entry_state NOT NULL DEFAULT 'open',
  invoice_id    uuid,                                   -- Phase 4
  created_at    timestamptz NOT NULL DEFAULT now(),
  updated_at    timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX ON time_entries (user_id, spent_date);
CREATE INDEX ON time_entries (project_id, spent_date);
CREATE UNIQUE INDEX one_running_timer_per_user ON time_entries (user_id) WHERE is_running;

CREATE TABLE approvals (
  id            uuid PRIMARY KEY,
  org_id        uuid NOT NULL REFERENCES organizations(id),
  user_id       uuid NOT NULL REFERENCES users(id),
  period_start  date NOT NULL,
  period_end    date NOT NULL,
  state         entry_state NOT NULL DEFAULT 'submitted', -- submitted|approved (reject deletes row)
  submitted_at  timestamptz NOT NULL DEFAULT now(),
  approved_by   uuid REFERENCES users(id),
  approved_at   timestamptz,
  UNIQUE (user_id, period_start)
);

CREATE TABLE audit_log (
  id            uuid PRIMARY KEY,
  org_id        uuid NOT NULL,
  actor_user_id uuid,
  action        text NOT NULL,                          -- e.g. 'entry.submit'
  entity_type   text NOT NULL,
  entity_id     uuid,
  data          jsonb,
  created_at    timestamptz NOT NULL DEFAULT now()
);
```

---

## 3. Core domain logic (`core` crate)

Modules & the functions an agent must implement + unit-test:

- **`duration`**: parse user input `"1:30"` and `"1.5"` → minutes (90); format minutes → `"1:30"`
  and decimal. Reject negatives / malformed.
- **`rounding`**: `round(minutes, increment, dir) -> minutes`. `increment=0` ⇒ identity.
  `nearest`: round to closest multiple (ties round up). `up`/`down`: ceil/floor to multiple.
  Property: result is a multiple of `increment`; `round(x,0,*) == x`; monotonic in x.
- **`totals`**: sum entries → day/week totals; billable vs non-billable split; per-project and
  per-task rollups. Totals use rounded minutes when the applicable rounding ≠ 0, and **the same
  function** is used by API responses, exports, and the UI.
- **`state`**: `can_transition(from, to, actor_role) -> bool` and allowed edges
  (`open→submitted→approved`, `submitted→open` via reopen/reject, `approved→submitted` reject).
  `invoiced` transitions are Phase 4 — reject them for now. Editing/deleting an entry is only
  allowed in `open`.
- **`money`**: cents + currency; never mix currencies in a sum (return error / grouped result).

No DB, no HTTP here. This crate should reach high line coverage.

---

## 4. API surface (`server` crate)

All under `/api`, JSON, cookie-authenticated. `org_id` derived from the session user. Return
problem+json on error. Write endpoints check the permission matrix (§6).

**Auth**
- `GET  /auth/login` → redirect to OIDC (or dev login if `DEV_LOGIN=1`)
- `GET  /auth/callback` → exchange code, upsert user by `oidc_subject`, set session cookie
- `POST /auth/logout`
- `GET  /api/me` → `{ user, org, role }`

**Clients** (read: any; write: admin)
- `GET /api/clients` · `POST /api/clients` · `GET/PATCH /api/clients/:id`

**Projects / tasks / assignments** (read: assigned members + admin/manager; write: admin)
- `GET /api/projects` (list w/ budget + spent + remaining) · `POST /api/projects`
- `GET/PATCH /api/projects/:id`
- `GET /api/projects/:id/tasks` · `PUT /api/projects/:id/tasks` (set enabled + overrides)
- `GET /api/projects/:id/assignments` · `POST` · `DELETE /assignments/:aid`
- `GET /api/tasks` · `POST /api/tasks`

**Time entries** (owner or admin)
- `GET  /api/time-entries?from=&to=&user_id=` → entries + computed rounded + totals
- `POST /api/time-entries` (manual entry) — body: project_id, task_id, spent_date, minutes, notes
- `PATCH /api/time-entries/:id` (only if `open`)
- `DELETE /api/time-entries/:id` (only if `open`)
- `POST /api/timers/start` (project_id, task_id, notes) → creates running entry (enforces one-per-user)
- `POST /api/timers/stop` → stops the running entry, writes minutes
- `GET  /api/timers/current`

**Approvals**
- `POST /api/approvals/submit` (user_id defaults to self, period_start) → sets entries in period to
  `submitted`, persists `rounded_minutes`, creates approvals row, locks the period
- `POST /api/approvals/:id/approve` (manager/admin) → entries → `approved`
- `POST /api/approvals/:id/reject` (manager/admin) → entries → `open`, delete approvals row
- `GET  /api/approvals?status=&period=&group=person`

**Reports**
- `GET /api/reports/time?from=&to=&group=client|project|task|person` → grouped totals
- `GET /api/reports/detailed?from=&to=&filters…` → row model
- `GET /api/reports/detailed/export?format=csv|xlsx|pdf&…` → file download; PDF is the per-
  client/per-project rounded timesheet

Keep DTOs in a shared module; the web SPA reuses the same shapes via generated or hand-written
types.

### 4.1 Harvest-compatible JSON API (MVP requirement)

A second, **Harvest-API-v2-shaped** surface under `/v2`, so existing Harvest clients
(`harvest-invoicer`, `harvest-exporter`, and any Harvest SDK) work against this tool with minimal
change. Follow the official shapes exactly — see
<https://help.getharvest.com/api-v2/> (esp. the Time Entries endpoint).

- **Auth:** accept `Authorization: Bearer <token>` **and** `Harvest-Account-Id: <org>` headers
  (plus `User-Agent`). Issue per-user API tokens (new `api_tokens` table: id, user_id, token_hash,
  name, created_at, last_used_at). Token scope = that user unless they're admin/manager.
- **Pagination envelope** (match Harvest exactly):
  ```json
  { "time_entries": [ … ], "per_page": 100, "total_pages": 1, "total_entries": 3,
    "page": 1, "next_page": null, "previous_page": null,
    "links": { "first": "…", "next": null, "previous": null, "last": "…" } }
  ```
  The array key is the resource name (`time_entries`, `projects`, `clients`, `tasks`, `users`).
- **Endpoints (read-only for MVP):**
  - `GET /v2/users/me`
  - `GET /v2/time_entries` — filters `user_id`, `client_id`, `project_id`, `task_id`, `from`,
    `to`, `is_billed`, `is_running`, `updated_since`; paginated.
  - `GET /v2/time_entries/:id`
  - `GET /v2/projects`, `GET /v2/projects/:id`
  - `GET /v2/clients`, `GET /v2/clients/:id`
  - `GET /v2/tasks`, `GET /v2/tasks/:id`
  - `GET /v2/users`
- **Field mapping** — emit Harvest's field names, backed by our model:

  | Harvest field | Source |
  |---|---|
  | `hours` | `minutes / 60` (decimal) |
  | `rounded_hours` | `rounded_minutes / 60` (falls back to computed round if not yet locked) |
  | `is_locked` | `state ∈ {submitted, approved, invoiced}` |
  | `locked_reason` | derived from `state` ("Approved", "Invoiced", …) |
  | `approval_status` | map `state`: open→`unsubmitted`, submitted→`pending_approval`, approved/invoiced→`approved` |
  | `is_billed` | `invoice_id IS NOT NULL` |
  | `is_running`, `timer_started_at` | from timer fields |
  | `billable` | entry `billable` |
  | `client{ id, name, currency }` · `project{ id, name, code }` · `task{ id, name }` · `user{ id, name }` | joins |
  | `spent_date`, `notes`, `created_at`, `updated_at` | direct |

- **Writes are Phase 2** — MVP exposes read only; that already lets `harvest-exporter`/
  `harvest-invoicer` pull from this tool.

> This `/v2` surface and the internal `/api` surface (§4) share the same handlers/DTO-mapping
> layer where possible — `/v2` is a Harvest-shaped view over the same data.

---

## 5. Web UI (`web` crate) — routes & behavior

Dioxus SPA calling `/api`. Routes:

- `/login` — OIDC button (+ dev login in dev).
- `/` → redirect to `/timesheet`.
- `/timesheet` — **Day / Week / Calendar** toggle.
  - Day: list of entries (project·task, notes, duration, timer Stop/Start, edit); day total.
  - Week: grid, row per project-task, editable per-day cells, daily + weekly totals, add-row.
  - Calendar: hourly week grid, entries as blocks.
  - New/edit entry dialog: searchable **project/task picker grouped by client** (currency in
    header), task, notes, duration, Start-timer vs manual.
  - Persistent running-timer widget in the header.
  - "Submit week for approval" button (disabled once submitted).
- `/projects` — list grouped by client with budget/spent/remaining; `/projects/:id` detail
  (metrics, per-task breakdown, team). Edit controls visible to admin only.
- `/approvals` — manager/admin: period + status filters, group-by-person, bulk approve.
- `/reports` — Time + Detailed tabs, filters, export buttons.
- `/settings` — admin: manage clients, projects, tasks, users/assignments.

State: fetch-on-navigate; optimistic updates for entry edits with rollback on error. No
localStorage requirement.

---

## 6. Permissions matrix

| Action | member | manager | admin |
|---|---|---|---|
| Track own time, edit own `open` entries | ✅ | ✅ | ✅ |
| View projects they're assigned to | ✅ | ✅ | ✅ |
| Submit own week | ✅ | ✅ | ✅ |
| View all users' time / reports | ❌ | ✅ | ✅ |
| Approve / reject timesheets | ❌ | ✅ | ✅ |
| Create/edit clients, projects, tasks, assignments | ❌ | ❌ | ✅ |
| Manage users / org settings | ❌ | ❌ | ✅ |

Enforce server-side on every write; the UI hides controls but is not the gate.

---

## 7. Rounding (concrete, Phase-1)

Phase 1 uses **org-level** rounding (`round_minutes`, `round_dir`); per-client/project overrides
are Phase 2. Algorithm (in `core::rounding`):

```
fn round(minutes, inc, dir):
    if inc <= 0: return minutes
    q = minutes / inc         # integer div
    r = minutes % inc
    match dir:
        Down:    return q*inc
        Up:      return (q + (r>0 ? 1:0)) * inc
        Nearest: return (r*2 >= inc ? q+1 : q) * inc   # ties → up
```

Applied per **entry**. When a week is submitted, each entry's `rounded_minutes` is computed and
**persisted** so later changes to org rounding don't rewrite locked history. Screen totals,
report responses, CSV/XLSX, and the PDF all call this same function (or read the persisted value
once locked). Exports print the rule, e.g. "Rounded to nearest 15 min."

---

## 8. Seed & migration

- Seed script: one org, one admin user (dev login), 2 clients (EUR + USD), 2 projects, a handful
  of tasks, and sample time entries across a week — enough to see day/week/calendar and reports.
- Harvest import is **out of scope for Phase 1** (Phase 2). Do not build it yet.

---

## 9. Testing

- **core:** unit tests for duration parsing, rounding (incl. property tests), totals, state edges.
- **server:** integration tests against a throwaway Postgres (testcontainers or a CI service):
  auth/session, permission matrix (each row above), timer one-per-user constraint, submit→lock
  (editing a submitted entry must 409), reports totals match `core`.
- **web:** Playwright e2e for the golden path: login → start/stop timer → add manual entry →
  see week total → submit → verify locked → export PDF (assert PDF magic bytes + rounded value).
- CI: `cargo test` + `cargo clippy -D warnings` + fmt + the e2e job.

---

## 10. Milestones (build in order)

**M0 — Scaffold.** Workspace, three crates, `docker-compose` (postgres), `.env.example`, CI
(build/clippy/fmt). *Done-when:* `docker compose up` boots Postgres and the server serves
`GET /health`.

**M1 — Core domain.** `duration`, `rounding`, `totals`, `state`, `money` with unit/property
tests. *Done-when:* `cargo test -p core` green with the rounding + state properties covered.

**M2 — Schema + data layer.** Migration `0001_init.sql`, sqlx setup, seed script. *Done-when:*
migrations apply cleanly and seed populates the sample org.

**M3 — Auth & users.** OIDC login + dev-login bypass, session cookie, `/api/me`, role loading.
*Done-when:* dev login yields an authenticated admin session; `/api/me` returns role.

**M4 — Clients/projects/tasks/assignments.** CRUD API + admin `/settings` UI, permission checks.
*Done-when:* an admin can create a client→project→tasks→assign a user via the UI; a member cannot.

**M5 — Time entries + timer + timesheet.** Entries API, timer start/stop (one-per-user),
`/timesheet` Day + Week + entry dialog + running widget. *Done-when:* a user can track via timer
and manual entry and see correct day/week totals.

**M6 — Calendar view.** `/timesheet` Calendar mode. *Done-when:* the week's entries render as
blocks with a correct week total.

**M7 — Approvals.** Submit-locks-period, approve/reject, `/approvals` dashboard. *Done-when:*
submitting locks entries (editing a submitted entry returns 409) and a manager can approve/reject.

**M8 — Reports & export.** Time + Detailed reports, CSV/XLSX/PDF export with rounding, per-
client/project PDF timesheet. *Done-when:* the PDF's hours equal the rounded values and match the
on-screen totals exactly.

**M8.5 — Harvest-compatible API.** `/v2` read endpoints (§4.1), API tokens + `Harvest-Account-Id`
auth, Harvest pagination envelope, field mapping. *Done-when:* a Harvest API client (or a curl
script mimicking `harvest-exporter`) can authenticate and page through `/v2/time_entries` with
`rounded_hours`/`is_locked`/`approval_status` correctly populated.

**M9 — Package & harden.** Dockerfiles, README (self-host guide), e2e golden path, clippy clean.
*Done-when:* a fresh `docker compose up` gives a working app and the e2e path passes.

---

## 11. Out of scope for this spec (later phases)

Invoicing & compliance packs, versioned budget periods, multi-currency override, SoW tracking,
WASM plugins, the export-API-as-product, Harvest import, expenses, mobile/desktop. Build none of
these in Phase 1 — but don't design the schema in a way that blocks them (the `org_id` FKs,
`entry_state` enum incl. `invoiced`, `invoice_id` column, and per-project `currency` are already
placed for them).