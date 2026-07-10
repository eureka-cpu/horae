# Harvest-Compatible REST API Contract

The Harvest-compatible API is a **read-only** REST surface mounted at
`/harvest/v2/*`. It mirrors the response shape of the
[Harvest API v2](https://help.getharvest.com/api-v2/) so that existing
Harvest-oriented tooling (for example `harvest-invoicer` and
`harvest-exporter`) can be pointed at a Horae deployment
(`https://horae.example.com/harvest`) and consume its data unchanged.

The router is defined in `crates/horae/src/harvest/mod.rs`, with response DTOs in
`crates/horae/src/harvest/types.rs` and the authenticating extractor in
`crates/horae/src/harvest/auth.rs`. It is merged onto the main Axum router in
`crates/horae/src/main.rs` under the Postgres-backed session layer.

## Scope

1. This API is **strictly read-only**. Only `GET` routes exist; there are no
   create, update, or delete endpoints.
1. All mutations in Horae go through the internal Dioxus `#[server]` functions
   (`crates/horae/src/server_fns.rs`), which the SPA uses directly. Those are a separate,
   session-authenticated surface and are **not** part of this contract.
1. All results are scoped to the authenticated user's organization (`org_id`).
   Horae is single-organization for now, but every query filters on `org_id`.

## Authentication

1. Authentication is **session-based** via the `tower-sessions` cookie set when
   a user signs in through the web app. Every endpoint requires a valid session;
   the request is resolved to a `user_id` and `org_id` by the `AuthUser`
   extractor.
1. Requests without a valid session are rejected with `401 Unauthorized`
   (`"No session"` when the session is absent, `"Not authenticated"` when the
   session carries no user, `"User not found"` when the user cannot be resolved).
1. `Authorization: Bearer <token>` authentication is **planned for Phase 2**
   (per the README) and is **not implemented**. The code comments in
   `crates/horae/src/harvest/auth.rs` note that a future iteration will add bearer-token
   support backed by an `api_tokens` table; no such table or code path exists
   today.

## Response Envelope

### Collection (list) responses

List endpoints return a Harvest-style paginated envelope. The collection is
placed under a **named key** matching the resource (for example `time_entries`,
`projects`, `clients`, `tasks`, `users`), alongside pagination metadata:

```json
{
  "<resource_name>": [ /* array of resource objects */ ],
  "per_page": 100,
  "total_pages": 3,
  "total_entries": 250,
  "page": 1,
  "next_page": 2,
  "previous_page": null,
  "links": {
    "first": "/harvest/v2/<resource>?page=1&per_page=100",
    "next": "/harvest/v2/<resource>?page=2&per_page=100",
    "previous": null,
    "last": "/harvest/v2/<resource>?page=3&per_page=100"
  }
}
```

Envelope field semantics (from `HarvestPagination` in
`crates/horae/src/harvest/types.rs`):

1. `<resource_name>` — array of resource objects; the key is the resource name.
1. `per_page` — page size actually used (see pagination rules below).
1. `total_pages` — computed as `ceil(total_entries / per_page)`, and is `1` when
   `total_entries` is `0`.
1. `total_entries` — total number of matching rows across all pages.
1. `page` — the current page number (1-based).
1. `next_page` — next page number, or `null` when on the last page.
1. `previous_page` — previous page number, or `null` when on the first page.
1. `links` — object with `first`, `next`, `previous`, and `last` URLs; `next`
   and `previous` are `null` at the respective boundaries.

### Single-resource responses

Endpoints that fetch a single resource by id return the bare resource object
(no envelope), for example a single `HarvestTimeEntry` or `HarvestProject`.

### Pagination rules

1. `page` defaults to `1` and is clamped to a minimum of `1`.
1. `per_page` defaults to `100` and is clamped to the range `1..=100`.
1. These rules apply uniformly to every list endpoint.

### Error responses

1. `401 Unauthorized` — missing or invalid session (see Authentication).
1. `404 Not Found` — a single-resource lookup by id matched no row in the
   caller's organization (`"Not found"`).
1. `500 Internal Server Error` — database or serialization failure
   (`"Internal error: <detail>"`).

## Endpoints

### `GET /harvest/v2/users/me`

Returns the authenticated user as a single `HarvestUser` object. No query
parameters.

### `GET /harvest/v2/time_entries`

Returns a paginated collection under the `time_entries` key. Ordered by
`spent_date` descending, then `created_at` descending.

Query parameters (all optional):

1. `from` — inclusive lower bound on `spent_date` (`YYYY-MM-DD`).
1. `to` — inclusive upper bound on `spent_date` (`YYYY-MM-DD`).
1. `user_id` — filter by user UUID.
1. `project_id` — filter by project UUID.
1. `is_running` — boolean; filter running vs. stopped entries.
1. `updated_since` — timestamp (`timestamptz`); entries updated at or after this
   time. Accepted by the API though not listed in the README.
1. `page` — page number (default `1`).
1. `per_page` — page size (default `100`, max `100`).

### `GET /harvest/v2/time_entries/{id}`

Returns a single `HarvestTimeEntry` by UUID, or `404` if not found in the
caller's organization.

### `GET /harvest/v2/projects`

Returns a paginated collection under the `projects` key. Ordered by `name`.

Query parameters (all optional):

1. `is_active` — boolean; filter by active state.
1. `client_id` — filter by client UUID.
1. `updated_since` — timestamp; projects created at or after this time. (Note:
   the underlying `projects` table has no `updated_at` column, so this filters
   on `created_at` and the emitted `updated_at` mirrors `created_at`.)
1. `page`, `per_page` — pagination (defaults `1` / `100`, max `100`).

### `GET /harvest/v2/projects/{id}`

Returns a single `HarvestProject` by UUID, or `404` if not found.

### `GET /harvest/v2/clients`

Returns a paginated collection under the `clients` key. Ordered by `name`.

Query parameters (all optional):

1. `is_active` — boolean; filter by active state.
1. `updated_since` — timestamp; clients created at or after this time. (The
   `clients` table has no `updated_at` column; `updated_at` mirrors
   `created_at`.)
1. `page`, `per_page` — pagination (defaults `1` / `100`, max `100`).

### `GET /harvest/v2/clients/{id}`

Returns a single `HarvestClient` by UUID, or `404` if not found.

### `GET /harvest/v2/tasks`

Returns a paginated collection under the `tasks` key. Ordered by `name`.

Query parameters (all optional):

1. `is_active` — boolean; filter by active state.
1. `page`, `per_page` — pagination (defaults `1` / `100`, max `100`).

Note: `updated_since` is accepted as a parameter but currently ignored for
tasks. The `tasks` table has no timestamp columns, so `created_at` and
`updated_at` are emitted as empty strings.

### `GET /harvest/v2/tasks/{id}`

Returns a single `HarvestTask` by UUID, or `404` if not found.

### `GET /harvest/v2/users`

Returns a paginated collection under the `users` key. Ordered by `name`.

Query parameters (all optional):

1. `is_active` — boolean; filter by active state.
1. `page`, `per_page` — pagination (defaults `1` / `100`, max `100`).

Note: `updated_since` is accepted as a parameter but currently ignored for
users. The `users` table has no `updated_at` column; `updated_at` mirrors
`created_at`.

## Resource Object Shapes

Field names and types below reflect the serialized DTOs in
`crates/horae/src/harvest/types.rs`. All ids are UUID strings. Durations are exposed as
**hours** (`f64`) and rates/amounts as **major currency units** (`f64`), even
though Horae stores minutes and cents internally.

### `HarvestTimeEntry`

1. `id` — string (UUID).
1. `spent_date` — string (`YYYY-MM-DD`).
1. `hours` — number; `minutes / 60`.
1. `rounded_hours` — number; rounded minutes / 60. For locked entries the stored
   `rounded_minutes` is used; otherwise rounding is computed from the
   organization's `round_minutes` / `round_dir` settings.
1. `notes` — string or `null`.
1. `is_locked` — boolean; `true` when the entry state is `submitted`,
   `approved`, or `invoiced`.
1. `locked_reason` — string or `null` (`"Pending Approval"`, `"Approved"`,
   `"Invoiced"`, else `null`).
1. `is_closed` — boolean; equal to `is_locked`.
1. `is_billed` — boolean; `true` when the entry has an `invoice_id`.
1. `is_running` — boolean.
1. `timer_started_at` — RFC 3339 timestamp string or `null`.
1. `billable` — boolean.
1. `budgeted` — boolean; `true` when the project's budget kind is not `none`.
1. `billable_rate` — number or `null` (user billable rate, in currency units).
1. `cost_rate` — number or `null` (user cost rate, in currency units).
1. `created_at`, `updated_at` — RFC 3339 timestamp strings.
1. `user` — `{ id, name }`.
1. `client` — `{ id, name }`.
1. `project` — `{ id, name, code }` (`code` may be `null`).
1. `task` — `{ id, name }`.
1. `approval_status` — string: `unsubmitted`, `pending_approval`, or `approved`
   (derived from the internal entry state).

### `HarvestProject`

1. `id` — string (UUID).
1. `name` — string.
1. `code` — string or `null`.
1. `is_active` — boolean.
1. `is_billable` — boolean; `false` only when the project type is
   `non_billable`.
1. `bill_by` — string: `Tasks` (time and materials), `Project` (fixed fee or
   retainer), else `none`.
1. `budget_by` — string: `person` (hours budget), `project_cost` (amount
   budget), else `none`.
1. `budget` — number or `null`; budget hours or budget amount depending on
   budget kind.
1. `starts_on`, `ends_on` — date strings or `null`.
1. `created_at`, `updated_at` — RFC 3339 timestamp strings (`updated_at` mirrors
   `created_at`).
1. `client` — `{ id, name }`.

### `HarvestClient`

1. `id` — string (UUID).
1. `name` — string.
1. `is_active` — boolean.
1. `address` — string or `null`.
1. `currency` — string (ISO currency code).
1. `created_at`, `updated_at` — RFC 3339 timestamp strings (`updated_at` mirrors
   `created_at`).

### `HarvestTask`

1. `id` — string (UUID).
1. `name` — string.
1. `is_active` — boolean.
1. `billable_by_default` — boolean.
1. `default_hourly_rate` — number or `null` (currency units).
1. `created_at`, `updated_at` — strings; currently empty (no timestamp columns
   on the `tasks` table).

### `HarvestUser`

1. `id` — string (UUID).
1. `first_name`, `last_name` — strings; split from the stored full name on the
   first space (`last_name` is empty when the name has no space).
1. `email` — string.
1. `is_active` — boolean.
1. `is_admin` — boolean; `true` when the user's org role is `admin`.
1. `cost_rate` — number or `null` (currency units).
1. `default_hourly_rate` — number or `null` (billable rate, currency units).
1. `created_at`, `updated_at` — RFC 3339 timestamp strings (`updated_at` mirrors
   `created_at`).
