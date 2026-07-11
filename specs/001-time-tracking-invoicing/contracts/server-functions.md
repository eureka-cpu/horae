# Server Functions Contract

This document is the interface contract for Horae's **internal Dioxus `#[server]`
functions** — the primary UI↔server API. The single-page app (SPA) calls these
for all data reads and mutations; Dioxus registers each one as an HTTP endpoint on
the Axum router automatically.

This is a **contract, not an implementation** — no function bodies are given.

## Conventions

1. **Transport & auth**: Every function is session-authenticated. The caller's
   identity is taken from the Postgres-backed cookie session (`tower_sessions`);
   IDs are never trusted from the client for the "acting user". Functions that
   read the session but find no user return `401`.
1. **Error type**: All functions return `Result<T, ServerFnError>`. Failures are
   modeled as `ServerFnError::ServerError { message, code, details }`, where
   `code` mirrors an HTTP status: `401` unauthenticated, `403` forbidden (role or
   assignment), `404` not found, `409` conflict (locked entry, timer already
   running), `500` internal/database error.
1. **ID encoding**: UUIDs cross the wire as `String` and are parsed server-side to
   `Uuid` (invalid strings yield a `500` "Invalid …" error). Dates cross as
   `String` in `YYYY-MM-DD` form and parse to `chrono::NaiveDate`. The "typed
   conceptually" column below shows the logical type.
1. **Roles**: The role hierarchy is `member` < `manager` < `admin` (stored as the
   `org_role` text `"member" | "manager" | "admin"`). "member" means any
   authenticated user. `require_manager()` accepts `manager` or `admin`;
   `require_admin()` accepts only `admin`.
1. **Units**: Durations are integer minutes; money is integer minor units (cents) +
   ISO currency code — never floats (per the domain invariants).
1. **(planned)** marks functions required by the spec (FR-001..FR-023) that are
   **not yet defined** in `crates/horae/src/server_fns.rs`.

______________________________________________________________________

## Auth

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `login` | `email: String`, `password: String` | `()` | `ServerFnError` — always `401`; this is a stub. Real login is the Axum route `POST /auth/dev-login` (dev) / OIDC (prod). | member (public) |
| `logout` | — | `()` | `ServerFnError` (`500` on session error) | member |
| `get_me` | — | `User` | `ServerFnError` (`401` no session, `404` user not found/inactive) | member |

Notes:

1. Interactive sign-in does **not** go through `login`; it is served by plain Axum
   routes (`GET /auth/login`, `POST /auth/dev-login`, `POST /auth/logout`) outside
   the Dioxus `#[server]` surface. Production uses OIDC; `DEV_LOGIN=1` enables a
   one-click admin login.
1. **Dev login** is the Axum `POST /auth/dev-login` handler, not a `#[server]`
   function. It is listed here because it is part of the UI↔server auth contract,
   but it satisfies FR-001 via the Axum surface rather than a server function.

______________________________________________________________________

## Time entries

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `list_time_entries` | `user_id: Option<Uuid>` (reserved; results scoped to session user), `project_id: Option<Uuid>`, `date_from: Option<Date>`, `date_to: Option<Date>`, `limit: Option<i64>` (default 50) | `Vec<TimeEntry>` | `ServerFnError` (`401`, `500`, invalid filter) | member (own entries) |
| `create_time_entry` | `project_id: Uuid`, `task_id: Uuid`, `spent_date: Date`, `minutes: i32`, `notes: Option<String>`, `billable: bool` | `TimeEntry` | `ServerFnError` (`401`, `403` not assigned to project, `500`) | member (must be assigned; admins bypass) |
| `update_time_entry` | `entry_id: Uuid`, `minutes: i32`, `notes: Option<String>`, `billable: bool` | `TimeEntry` | `ServerFnError` (`409` entry not found or not in `open` state) | member (own, open entries) |
| `delete_time_entry` | `entry_id: Uuid` | `()` | `ServerFnError` (`409` entry not found or not in `open` state) | member (own, open entries) |
| `start_timer` | `project_id: Uuid`, `task_id: Uuid`, `notes: Option<String>` | `TimeEntry` | `ServerFnError` (`409` a timer is already running) | member |
| `stop_timer` | `entry_id: Uuid` | `TimeEntry` | `ServerFnError` (`404` no running timer for this entry) | member (own) |
| `get_current_timer` | — | `Option<TimeEntry>` | `ServerFnError` (`401`, `500`) | member |

Notes:

1. **FR-004** (one running timer per user) is enforced both by `start_timer`
   (returns `409`) and by a DB partial unique index.
1. **FR-015 / edit-lock**: `update_time_entry` and `delete_time_entry` succeed only
   while the entry is in the `open` state; once an entry is `submitted`, `approved`,
   or attached to an invoice it is locked (returns `409`).
1. `stop_timer` computes elapsed minutes exactly from `started_at` (minimum 1
   minute), satisfying FR-003 / FR-023.

______________________________________________________________________

## Clients

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `list_clients` | `include_inactive: bool` | `Vec<Client>` | `ServerFnError` (`500`) | member |
| `create_client` | `name: String`, `currency: String`, `address: Option<String>`, `tax_id: Option<String>` | `Client` | `ServerFnError` (`403` non-manager) | manager |
| `update_client` | `client_id: Uuid`, `name: String`, `currency: String`, `address: Option<String>`, `tax_id: Option<String>` | `Client` | `ServerFnError` (`403`, `404`) | manager |
| `set_client_active` | `client_id: Uuid`, `active: bool` | `Client` | `ServerFnError` (`403`, `404`) | manager |

Notes:

1. `list_clients` returns only `active = true` rows by default so inactive clients
   are not selectable for new work (FR-011); the management view passes
   `include_inactive = true` to also list deactivated clients for reactivation.
1. Per FR-008 client create/edit/deactivate are gated at **manager** (managers or
   admins).

______________________________________________________________________

## Projects

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `list_projects` | `client_id: Option<Uuid>` (reserved; not yet filtered), `include_inactive: bool` | `Vec<Project>` | `ServerFnError` (`500`) | member |
| `create_project` | `client_id: Uuid`, `name: String`, `project_type: String`, `currency: String`, `budget_kind: String` | `Project` | `ServerFnError` (`403` non-manager) | manager |
| `update_project` | `project_id: Uuid`, `name: String`, `project_type: String`, `currency: String`, `budget_kind: String` | `Project` | `ServerFnError` (`403`, `404`) | manager |
| `set_project_active` | `project_id: Uuid`, `active: bool` | `Project` | `ServerFnError` (`403`, `404`) | manager |

Notes:

1. `project_type` and `budget_kind` are Postgres enums bound as text; the billing
   method / budget rate fields of FR-009 map onto these plus `budget_amount_cents`
   / `budget_minutes` on the row.
1. Per FR-009 project create/edit/deactivate are gated at **manager**. The
   management view passes `include_inactive = true` to `list_projects` to include
   inactive projects for reactivation. The `client_id` filter argument on
   `list_projects` is accepted but not yet applied.

______________________________________________________________________

## Tasks

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `list_tasks` | `project_id: Option<Uuid>` (accepted for back-compat; tasks are org-level) | `Vec<Task>` (active only) | `ServerFnError` (`500`) | member |
| `list_project_tasks` | `project_id: Uuid` | `Vec<Task>` (linked via `project_tasks`) | `ServerFnError` (`401`, `500`) | member |
| `create_task` | `name: String`, `billable_default: bool` | `Task` | `ServerFnError` (`403` non-manager) | manager |
| `update_task` | `task_id: Uuid`, `name: String`, `billable_default: bool`, `default_rate_cents: Option<i64>` | `Task` | `ServerFnError` (`403`, `404`) | manager |
| `set_task_active` | `task_id: Uuid`, `active: bool` | `Task` | `ServerFnError` (`403`, `404`) | manager |
| `link_project_task` | `project_id: Uuid`, `task_id: Uuid` | `()` | `ServerFnError` (`403`, `404`) | manager |

Notes:

1. Tasks are **org-level** in the current schema; the per-project relationship is
   the `project_tasks` join table surfaced by `list_project_tasks`.
1. Per FR-010 task create/edit/deactivate and project linking are gated at
   **manager**. `link_project_task` inherits the task's `billable_default` /
   `default_rate_cents` onto the new `project_tasks` row and is idempotent
   (`ON CONFLICT DO NOTHING`).

______________________________________________________________________

## Assignments

Supporting surface for FR-005/FR-006 (a user may only log time on projects they are
assigned to). Not called out in the prompt's grouping but part of the real contract.

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `list_assignments` | `project_id: Uuid` | `Vec<Assignment>` | `ServerFnError` (`401`, `500`) | member |
| `create_assignment` | `project_id: Uuid`, `user_id: Uuid`, `role: String` | `Assignment` | `ServerFnError` (`403` non-admin) | admin |
| `delete_assignment` | `assignment_id: Uuid` | `()` | `ServerFnError` (`403` non-admin) | admin |

______________________________________________________________________

## Invoices

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `list_invoices` | `status: Option<String>` | `Vec<Invoice>` | `ServerFnError` | manager (planned; currently unauthenticated stub) |
| `get_invoice` **(planned)** | `invoice_id: Uuid` | `Invoice` (with line items) | `ServerFnError` (`403`, `404`) | manager |
| `generate_invoice_from_time` **(planned)** | `client_id: Uuid`, `period_from: Date`, `period_to: Date` | `Invoice` | `ServerFnError` (`403`, `404` nothing to invoice, `409` time already billed) | manager |
| `update_invoice_status` **(planned)** | `invoice_id: Uuid`, `status: "draft" \| "sent" \| "paid" \| "void"` | `Invoice` | `ServerFnError` (`403`, `404`, `409` illegal transition) | manager |

Notes:

1. **Invoicing is Phase 4 and largely (planned)**. The current `list_invoices` is a
   stub: it takes `status`, ignores it, does **not** check the session, and always
   returns an empty `Vec` because the `invoices` table does not exist yet.
1. **(planned)** functions above implement FR-012..FR-015: generate a draft invoice
   from a client's billable, un-invoiced time so totals reconcile exactly (FR-012,
   FR-023); mark covered entries invoiced so they cannot be double-billed (FR-013);
   carry number/issue date/due date/total across the draft→sent→paid / void
   lifecycle (FR-014); and lock invoiced time against edit/delete (FR-015).

______________________________________________________________________

## Reports

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `report_time` | `from: Date`, `to: Date`, `group_by: "project" \| "task" \| "client" \| "person"` | `Vec<ReportRow>` (`label`, `total_minutes`, `rounded_minutes`, `billable_minutes`) | `ServerFnError` (`401`, `500`, invalid date) | member (planned: manager) |
| `report_detailed` | `from: Date`, `to: Date` | `Vec<DetailedReportRow>` | `ServerFnError` (`401`, `500`, invalid date) | member (planned: manager) |

Export links (not `#[server]` functions):

1. Report/invoice **export** is served by plain Axum routes (CSV / XLSX via
   `reports.rs`), not by server functions, so the browser can download a file
   directly. Exported totals reconcile exactly with the on-screen figures
   (FR-016, FR-023, SC-007).
1. **(planned)**: FR-016 grants **managers** reporting; current `report_time` /
   `report_detailed` require only an authenticated member. The `"client"` grouping
   currently uses project name as a proxy until a clients join is added.

______________________________________________________________________

## Approvals

Weekly submit/approve workflow (milestone M7). Not a spec FR (the spec defers a
formal approval step), but part of the real contract and the entry-locking model.

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `submit_week` | `week_start: Date` | `Approval` | `ServerFnError` (`404` no open entries) | member (own week) |
| `list_approvals` | `status: Option<String>` | `Vec<Approval>` | `ServerFnError` (`403` non-manager) | manager |
| `approve_submission` | `approval_id: Uuid` | `Approval` | `ServerFnError` (`403`, `404` not in `submitted` state) | manager |
| `reject_submission` | `approval_id: Uuid` | `()` | `ServerFnError` (`403`, `404`) | manager |

______________________________________________________________________

## Admin / Users

| Function | Inputs | Output | Errors | Required role |
|---|---|---|---|---|
| `list_users` | — | `Vec<User>` (active only) | `ServerFnError` (`500`) | member (planned: admin) |
| `create_user` **(planned)** | `email: String`, `name: String`, `role: "member" \| "manager" \| "admin"` | `User` | `ServerFnError` (`403`, `409` email exists) | admin |
| `set_user_role` **(planned)** | `user_id: Uuid`, `role: "member" \| "manager" \| "admin"` | `User` | `ServerFnError` (`403`, `404`) | admin |
| `set_user_active` **(planned)** | `user_id: Uuid`, `active: bool` | `User` | `ServerFnError` (`403`, `404`) | admin |

Notes:

1. **(planned)**: FR-002 requires admins to create users, assign roles, and
   deactivate accounts (deactivated users cannot sign in). The current code exposes
   only `list_users`, which returns active users and does **not** yet require the
   admin role. User creation today happens via the CLI (`user create`) rather than a
   server function. Deactivation must preserve historical entries (edge case in the
   spec).
