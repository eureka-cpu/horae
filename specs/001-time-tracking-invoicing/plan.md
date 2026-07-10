# Implementation Plan: Time Tracking & Invoicing

**Branch**: `001-time-tracking-invoicing` | **Date**: 2026-07-10 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-time-tracking-invoicing/spec.md`

## Summary

Deliver the Horae MVP: a self-hostable, single-organization time-tracking and invoicing web app with a sandboxed plugin system. The approach is a **Dioxus fullstack** application (one Rust crate, `server` and `web` build targets) whose data mutations run through Dioxus `#[server]` functions on an Axum server, backed by PostgreSQL via `sqlx`. Correctness-critical domain logic (duration parsing, rounding, money, totals, entry/invoice state) lives in a dependency-light `horae-core` crate so totals are exact (SC-002/SC-007). Plugins are portable WASM modules hosted by `extism`, invoked on business events and confined to explicitly granted host capabilities.

Much of P1–P4 already exists in the codebase; the plan covers completing those flows to the spec and building the plugin subsystem (P5), which is currently unimplemented.

## Technical Context

**Language/Version**: Rust (edition 2024); frontend compiled to WASM via Dioxus.

**Primary Dependencies**: `dioxus` 0.7 (fullstack + router), `axum` 0.8, `tokio`, `sqlx` 0.8 (Postgres), `tower-sessions` + `tower-sessions-sqlx-store`, `openidconnect` (production auth), `clap` (CLI), `chrono`, `uuid` (v7), `csv` + `rust_xlsxwriter` (spreadsheet/CSV export), `typst` (invoice/timesheet PDF rendering, fonts from nixpkgs), `extism` 1 (plugin host — to add). Toolchain and packaging via Nix (`fenix`, `blueprint`, `treefmt-nix`).

**Storage**: PostgreSQL 15+. Migrations via `sqlx` (`crates/horae/migrations/`). Sessions persisted in Postgres.

**Testing**: `cargo test -p horae-core` (pure unit tests); `cargo test -p horae --features server` integration tests using `#[sqlx::test]` (throwaway DB per test, `#[serial]`); `nix flake check` runs formatting + a NixOS e2e VM test.

**Target Platform**: Linux server (self-hosted; NixOS module + systemd) for the backend; modern desktop browsers (WASM SPA) for the UI.

**Project Type**: Web application — Dioxus fullstack (shared UI + server functions in one crate) plus a pure-domain library crate.

**Performance Goals**: Interactive UI (live-incrementing timer); list/report views under 2 s at target scale (SC-005); plugin dispatch within 1 s and never blocking the core action (SC-006).

**Constraints**: Totals must be exact — durations stored as integer minutes, money as integer minor units + ISO currency code, never floats (SC-002/SC-007). UUID v7 primary keys. Postgres-only. `horae-core` must not depend on `sqlx`/`axum`/`dioxus`. Plugins run sandboxed with no direct datastore writes.

**Scale/Scope**: Single organization; ~50 active users and ~100k time entries per deployment (SC-005). Twelve routes (dashboard, time, timesheet, clients, projects, invoices, approvals, reports, admin, settings, auth).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Checked against the ratified project constitution (`.specify/memory/constitution.md`, **v1.0.0**):

- **I. Exactness (non-negotiable)**: integer minutes for time, integer minor units + ISO currency for money — no floats. ✅ Enforced via `horae-core`.
- **II. Domain purity**: correctness-critical logic isolated in `horae-core` with no I/O deps. ✅ Preserved.
- **III. Single datastore**: PostgreSQL only, UUID v7 keys, `org_id` FKs kept for later multi-org. ✅ Matches the schema.
- **IV. Mutations through server functions**: all writes via `#[server]` functions; the Harvest API stays read-only. ✅ Matches the architecture.
- **V. Reproducible builds & formatting gate**: Nix dev shell + `nix fmt`/`nix flake check` green. ✅ In place.

No violations to justify (Complexity Tracking empty).

## Project Structure

### Documentation (this feature)

```text
specs/001-time-tracking-invoicing/
├── plan.md              # This file
├── research.md          # Phase 0 — key technical decisions
├── data-model.md        # Phase 1 — entities, relationships, state machines
├── quickstart.md        # Phase 1 — runnable validation guide
├── contracts/           # Phase 1 — interface contracts
│   ├── server-functions.md
│   ├── harvest-api.md
│   ├── cli.md
│   └── plugin-interface.md
└── tasks.md             # Phase 2 — created by /speckit-tasks (not here)
```

### Source Code (repository root)

```text
Cargo.toml               # virtual workspace root (members: crates/horae, crates/core)
crates/
├── core/                # horae-core: pure domain (duration, rounding, money, totals, state)
└── horae/               # the Dioxus fullstack app (server + web targets)
    ├── Cargo.toml
    ├── Dioxus.toml
    ├── migrations/      # sqlx SQL migrations (0001_init.sql, …)
    ├── templates/       # Typst document templates: invoice.typ (+ timesheet.typ)  [NEW]
    ├── assets/css/horae.css   # design system
    ├── tests/           # #[sqlx::test] integration tests
    └── src/
        ├── main.rs      # cfg-gated entry points: server (Axum+CLI) / web (WASM) / stub
        ├── cli.rs       # clap CLI (serve, migrate, seed, user)          [server]
        ├── config.rs    # AppConfig from env                             [server]
        ├── db.rs        # PgPool + migrations                            [server]
        ├── state.rs     # global AppState (OnceCell): pool, plugins       [server]
        ├── auth/        # sessions, OIDC, DEV_LOGIN bypass               [server]
        ├── server_fns.rs # #[server] functions (CRUD, timer, auth, reports)
        ├── harvest/     # read-only Harvest-compatible REST API          [server]
        ├── reports.rs   # CSV/XLSX export Axum handlers                  [server]
        ├── seed.rs      # demo-data seeder                               [server]
        ├── models/      # user, client, project, task, time_entry, invoice
        ├── route.rs     # Routable Route enum (SPA routes)
        ├── app.rs       # root component + Router
        ├── pages/       # dashboard, time, timesheet, clients, projects, invoices, approvals, reports, admin, settings, auth
        ├── components/  # nav, sidebar, timer_widget, table, form, badge
        └── plugin/      # NEW (P5): registry, host functions, events, manifest [server]

nix/                     # blueprint flake tree: package, devshell, module, checks, formatter
nixos/ → nix/modules/nixos/horae.nix   # NixOS service module
```

**Structure Decision**: Keep the two-crate workspace — pure `horae-core` (correctness) + the `horae` fullstack app at `crates/horae/`. P1–P4 extend the existing `crates/horae/src/models/`, `server_fns.rs`, `pages/`, and `harvest/` code to fully satisfy the spec; P5 adds a new `crates/horae/src/plugin/` module and wires event dispatch into `server_fns.rs` after DB writes, with plugin handles held in `AppState`.

## Complexity Tracking

No constitution violations require justification. One notable subsystem — the WASM plugin host (`extism`) — is intentional and central to the spec (User Story 5), not incidental complexity; its sandboxing is a requirement (FR-020/FR-021), not an optional layer.
