# Phase 0 Research: Time Tracking & Invoicing

The spec is intentionally implementation-free; this document records the technical decisions that turn it into a buildable plan. Most were already pinned by `SPEC.md` and the existing codebase — captured here with rationale and the alternatives considered.

## Decision: Dioxus fullstack, single crate, feature-gated

- **Decision**: One `horae` app crate with `server` and `web` feature targets (three `cfg`-gated `main()`s), plus a pure `horae-core` library crate. All data mutations go through Dioxus `#[server]` functions on an Axum server.
- **Rationale**: Shared Rust types across client and server; type-safe calls without hand-written REST plumbing; `dx serve` builds both targets with hot reload. Isolating correctness logic in `horae-core` keeps totals testable without I/O.
- **Alternatives considered**: Separate Axum REST backend + standalone Dioxus SPA (SPEC.md's original §0 sketch) — more explicit boundary but more boilerplate and duplicated types; rejected in favor of the fullstack model already in the tree. A non-Rust frontend — rejected (loses shared types, single-language goal).

## Decision: Authentication via OIDC + session, with a dev bypass

- **Decision**: Production auth is OIDC (`openidconnect`); a `DEV_LOGIN=1` flag enables a one-click seeded-admin login for local development. Sessions are cookie-based, stored in Postgres (`tower-sessions` + `tower-sessions-sqlx-store`). Authorization is role-based (admin/manager/member).
- **Rationale**: Self-hosted operators typically already run an identity provider; delegating authentication avoids owning password storage/reset/MFA in a small tool. The dev bypass keeps local iteration fast.
- **Alternatives considered**: Local email + password (argon2id), as sketched in `PLAN.md` — reasonable and still compatible with the spec (which is auth-mechanism-agnostic, FR-001), but superseded by OIDC to avoid credential-management burden. Retained as a possible future local-auth mode; the spec's Assumptions leave the mechanism open.

## Decision: Exact time and money representation

- **Decision**: Durations stored as **integer minutes**, money as **integer minor units (cents) + ISO 4217 currency code**; all rounding/totalling done in `horae-core`. UUID v7 primary keys.
- **Rationale**: Guarantees SC-002/SC-007 (totals reconcile exactly) and FR-023; floats accumulate rounding error. v7 IDs are time-ordered for natural sorting and index locality.
- **Alternatives considered**: Floating-point hours/amounts — rejected (drift). Arbitrary-precision decimals — unnecessary given fixed minor units; adds dependency weight to `core`.

## Decision: Sandboxed WASM plugins via extism

- **Decision**: Plugins are WASM modules loaded at startup by `extism`. Each ships a `plugin.toml` (name, version, subscribed hooks). The host exposes a fixed set of capabilities (`horae_log`, read-only `horae_db_query`, `horae_http_post`, `horae_config_get`). On a business event, the registry dispatches to all subscribed plugins concurrently; results are isolated and time-bounded. Plugins may return a structured dashboard-widget spec.
- **Rationale**: WASM gives language-agnostic authoring and a strong sandbox by default (FR-020); extism provides ergonomic host functions and PDKs. Concurrent dispatch with timeouts satisfies FR-021 and SC-006 (never block the core action).
- **Alternatives considered**: Native dynamic-library plugins — full host access, no sandbox; rejected (FR-020). Raw `wasmtime`/WASI — viable but reimplements the host-function/PDK ergonomics extism already provides. Out-of-process webhooks only — less capable, no in-process widgets.

## Decision: No timesheet-approval workflow in v1

- **Decision**: Billable, un-invoiced time is directly invoiceable; there is no submit→approve gate before billing in this feature.
- **Rationale**: Matches `PLAN.md` and keeps the MVP focused (spec Assumptions). The schema keeps room for a richer entry lifecycle later (an `entry_state` enum can add `submitted`/`approved` without migration churn), and an "approvals" surface may be layered on in a future feature.
- **Alternatives considered**: Full approval lifecycle now (as `SPEC.md` and some scaffolding hint at) — deferred to avoid scope creep; revisit via a follow-up spec.

## Decision: Persistence, migrations, packaging, CI

- **Decision**: PostgreSQL 15+ via `sqlx`; migrations in `migrations/*.sql` applied by `horae migrate run` (and eagerly on `serve`). Exports use `csv`/`rust_xlsxwriter`. Toolchain and builds via Nix (`fenix` toolchain, `numtide/blueprint`), formatted by `treefmt`, checked by `nix flake check` (formatting + a NixOS e2e VM test).
- **Rationale**: Postgres is pinned by `SPEC.md`; sqlx gives async access and optional compile-time query checking. Nix gives reproducible dev shells, packages, and a deployable NixOS module.
- **Alternatives considered**: SQLite — explicitly excluded by `SPEC.md` for Phase 1. An ORM (SeaORM/Diesel) — rejected in favor of sqlx's explicit SQL and migration model.

## Resolved unknowns

No `NEEDS CLARIFICATION` markers remained in the spec. The five open scope questions were settled with documented defaults in the spec's Assumptions (single-org, self-hosted, credential/role auth, no v1 approval workflow, per-client currency) and are reflected in the decisions above.
