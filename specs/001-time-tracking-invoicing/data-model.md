# Phase 1 Data Model: Time Tracking & Invoicing

Derived from the spec's Key Entities and `SPEC.md` §2. Everything is scoped to a single `organizations` row, but every table keeps an `org_id` FK so multi-org is a later flip. Primary keys are UUID v7. Time is stored as **integer minutes**; money as **integer minor units (cents) + ISO 4217 currency code** — never floats.

## Enumerations

- **org_role**: `admin` | `manager` | `member` — a user's organization-wide role (authorization).
- **project_type**: `time_and_materials` | `fixed_fee` | `non_billable` | `retainer` — the project's billing method.
- **budget_kind**: `none` | `amount` | `hours` — how a project's budget is expressed.
- **entry_state**: `open` | `invoiced` for v1. (`submitted`/`approved` are reserved for a future approval workflow — see research.md — and are intentionally unused now.)
- **invoice_status**: `draft` | `sent` | `paid` | `void`.
- **round_dir**: `nearest` | `up` | `down` — rounding direction for optional time rounding.

## Entities

### Organization *(singleton for v1)*

- **Fields**: `id`, `name`, `default_currency` (char(3)), `week_start` (1 = Monday), `round_minutes` (0 = none), `round_dir`, `created_at`, plus **invoice-branding settings** (provider identity, bank/payment details, logo, default invoice template) used when rendering documents (FR-025).
- **Relationships**: owns all other records via `org_id`.

### User

- **Fields**: `id`, `org_id`, `email` (unique), `name`, `oidc_subject` (unique, null until first OIDC login), `org_role`, `cost_rate_cents` (optional), `billable_rate_cents` (optional), `active`, `created_at`.
- **Relationships**: owns `TimeEntry` records; may be assigned to projects.
- **Rules**: deactivating (`active = false`) blocks sign-in but preserves history (FR-002, edge case).

### Client

- **Fields**: `id`, `org_id`, `name`, `currency` (char(3)), `address`, `tax_id`, `active`, `created_at`.
- **Relationships**: owns `Project` records; billed via `Invoice`.
- **Rules**: `currency` is the client's single currency and drives its invoices (Assumptions). Inactive clients are hidden from new project/entry pickers.

### Project

- **Fields**: `id`, `org_id`, `client_id`, `code`, `name`, `project_type`, `currency` (defaults from client), `starts_on`, `ends_on`, `budget_kind`, `budget_amount_cents` (when `budget_kind = amount`), `active`, `created_at`.
- **Relationships**: belongs to a `Client`; has `Task` memberships, `Assignment`s, and `TimeEntry` records.
- **Rules**: inactive projects are not selectable for new time entries (FR-011).

### Task

- **Fields**: `id`, `org_id`, `name`. Enabled on a project through a **project-task** membership carrying `billable` (flag) and an optional `billable_rate_cents` override.
- **Relationships**: many-to-many with `Project` via project-task membership; referenced by `TimeEntry`.
- **Rules**: a time entry's task must be a member of the entry's project.

### Assignment

- **Fields**: `id`, `org_id`, `project_id`, `user_id`, per-assignment role, optional rate override.
- **Relationships**: links a `User` to a `Project`.
- **Rules**: governs which projects a member may log time against and rate overrides for billing.

### Time Entry

- **Fields**: `id`, `org_id`, `user_id`, `project_id`, `task_id`, `spent_on` (date), `started_at`, `ended_at` (null = running), `duration_minutes`, `notes`, `billable`, `entry_state`, `invoice_id` (null until invoiced), `created_at`, `updated_at`.
- **Relationships**: belongs to one `User`, `Project`, `Task`; optionally to one `Invoice`.
- **Rules**:
  1. At most one entry per user may have `ended_at = null` (one running timer — FR-004).
  1. On stop, `duration_minutes` is computed exactly from `started_at`/`ended_at` (FR-003, FR-023).
  1. Once `entry_state = invoiced` / `invoice_id` set, the entry is immutable unless removed from the invoice (FR-015).
  1. Only `billable = true`, `invoice_id = null` entries are eligible for invoicing (FR-012).

### Invoice

- **Fields**: `id`, `org_id`, `client_id`, `number`, `status`, `issued_on`, `due_on`, `currency`, `total_cents`, `created_at`.
- **Line items**: each line references the source billable time (task/description, `minutes`, `rate_cents`, `amount_cents`); the invoice `total_cents` equals the exact sum of its lines (FR-012, FR-023).
- **Relationships**: belongs to a `Client`; covers a set of `TimeEntry` records.
- **Rules**: generating an invoice marks its entries `invoiced` so the same time cannot be billed twice (FR-013).
- **Document**: the invoice PDF is a *derived artifact* rendered reproducibly from a template + the organization's branding settings (FR-025); the invoice and line records remain the authoritative data.

### Plugin *(planned — User Story 5)*

- **Fields**: `name`, `version`, subscribed `hooks[]`, `enabled`, plugin-scoped `config`.
- **Relationships**: reacts to `Event`s; may contribute dashboard widgets.
- **Rules**: sandboxed; only granted host capabilities; no direct datastore writes (FR-020). Loaded from `{dataDir}/plugins/`.

### Event *(runtime, not persisted)*

- **Kinds**: `time_entry_created`, `time_entry_stopped`, `invoice_created`, `invoice_sent`, `user_logged_in`.
- **Payload**: JSON snapshot of the relevant record; delivered to subscribed plugins (FR-019). See `contracts/plugin-interface.md`.

## State machines

### Time Entry

```text
running (ended_at = null)  ──stop──▶  stopped (open)  ──include on invoice──▶  invoiced
      │                                     │
   manual entry starts directly in "stopped (open)"
```

- `running → stopped`: sets `ended_at`, computes `duration_minutes`.
- `stopped(open) → invoiced`: sets `invoice_id`, `entry_state = invoiced` (only if billable).
- `invoiced → open`: only by removing the entry from its invoice (or voiding the invoice).

### Invoice

```text
draft ──▶ sent ──▶ paid
  │        │
  └──▶ void ◀──┘
```

- `draft → sent → paid`: normal lifecycle.
- `draft | sent → void`: cancels the invoice; its entries return to `open` (un-invoiced) so they can be re-billed.

## Cross-cutting validation

- All monetary and duration aggregates are computed in `horae-core` and MUST equal the exact sum of their parts (FR-023, SC-002/SC-007).
- Currency on an invoice matches its client's currency; mixing currencies on one invoice is not allowed (Assumptions).
- Inactive `Client`/`Project`/`Task` remain linked to historical records but are excluded from new-entry pickers (FR-011).
