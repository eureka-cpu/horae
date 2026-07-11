# AGENTS.md

This file provides guidance to coding agents (Claude Code and others) when working with code in this repository. `CLAUDE.md` is a symlink to this file.

Horae is a self-hostable time tracker (a Harvest/Kimai alternative) built as a Rust + [Dioxus](https://dioxuslabs.com/) fullstack app (SSR + WASM) on PostgreSQL/Axum.

## Commands

Work inside the Nix dev shell; a running PostgreSQL is required for anything touching the database.

```sh
nix develop            # dev shell: rust toolchain, dx (dioxus-cli), sqlx-cli, postgres, wasm-pack
nix run .#postgres     # boot a NixOS VM running PostgreSQL (forwards host :5432, :2222)
```

`DATABASE_URL` defaults to `postgres://localhost/horae`.

### Build & run

The app crate lives at `crates/horae/` (the repo root is a virtual workspace). It is **feature-gated** — `crates/horae/src/main.rs` has three `cfg`-selected `main()`s, and default features are empty. Always pick a feature or use `dx`, and select the crate with `-p horae`:

```sh
cargo build -p horae --features server          # server binary + CLI
cd crates/horae && DEV_LOGIN=1 DATABASE_URL=… dx serve   # dev server (dx runs where Dioxus.toml is), hot reload on :8080
cargo run -p horae --features server -- <subcommand>     # run the server binary directly
```

CLI subcommands: `serve`, `migrate run`, `migrate reset --confirm`, `seed`, `user list`, `user create --email … --name … --role …`.

First run: `… -- migrate run`, then `… -- seed`, then `dx serve`; open http://localhost:8080/auth/login and "Sign in as Admin" (needs `DEV_LOGIN=1`).

### Test & lint

```sh
cargo test -p horae-core                        # pure domain unit tests (no DB, no features)
DATABASE_URL=… cargo test -p horae --features server     # integration tests (need Postgres with CREATEDB)
DATABASE_URL=… cargo test -p horae --features server <name>   # a single test
cargo clippy -p horae --features server
nix fmt                                         # treefmt: rustfmt, taplo, nixpkgs-fmt, mdformat
```

Integration tests (`crates/horae/tests/integration.rs`) use `#[sqlx::test]` — each spins up a throwaway database, so the DB role needs `CREATEDB` — and are marked `#[serial]`. `nix build` builds the package; `nix flake check` runs the formatting check plus a full NixOS e2e test.

## Architecture

**One app crate (`crates/horae/`), two build targets, feature-gated.** `crates/horae/src/main.rs` defines three `main()`s behind `cfg`: `server` (Axum + Tokio + the CLI), `web` (`dioxus::launch`, compiled to WASM), and a stub that errors if neither feature is set. Server-only modules (`auth`, `cli`, `config`, `db`, `harvest`, `reports`, `seed`, `state`) are `#[cfg(feature = "server")]`; the shared UI modules (`app`, `route`, `pages`, `components`, `server_fns`, `models`, `error`) compile for both targets. This is why a bare `cargo build`/`test` (empty default features) won't do what you expect.

**The `core` crate (`horae-core`) is pure domain logic** — duration parsing, rounding, money, totals, the entry state machine — with no I/O dependencies (only serde/uuid/chrono/thiserror). Correctness-critical code belongs here and is unit-tested in isolation; SPEC.md §1 forbids sqlx/axum/dioxus deps in `core`.

**The server layers custom Axum routes on top of the Dioxus fullstack router** (`Commands::Serve` in `main.rs`): it calls `.serve_dioxus_application()`, then `.merge`s `/health`, CSV/XLSX export (`reports.rs`), the auth router (`auth::router()`), and the read-only Harvest-compatible API (`harvest::router()`, `/harvest/v2/*`), all under a Postgres-backed session layer. So there are **two API surfaces**:

- Dioxus `#[server]` functions in `server_fns.rs` — session-authenticated; the SPA uses these for all mutations.
- Plain Axum routes — health, exports, auth, and the read-only Harvest v2 API.

**Shared state**: `state.rs` holds a global `AppState` in a `OnceCell`, initialized once at startup with the `PgPool`; server fns and auth read from it. The pool is created and migrations applied eagerly on `serve`.

**Auth**: production uses OIDC (`openidconnect`); `DEV_LOGIN=1` enables a one-click admin login that bypasses OIDC (see `auth/`). Sessions are cookie-based, persisted in Postgres.

## Domain invariants (from SPEC.md — do not violate)

- Durations are stored as **integer minutes**; money as **integer minor units (cents) + ISO currency code** — never floats.
- Primary keys are **UUID v7** (time-ordered).
- **PostgreSQL only** (no SQLite). Migrations live in `crates/horae/migrations/` and apply via `sqlx` / `migrate run`.
- Single organization for now, but every table keeps an `org_id` FK so multi-org is a later flip.

`SPEC.md` is the authoritative Phase-1 build spec (schema, milestones, API contract). `DESIGN.md` is the design system (Invoicer aesthetic; tokens in `crates/horae/assets/css/horae.css`; components are one-per-file `#[component]` functions using `use_signal`/`use_resource`, with no global mutable UI state).

## Conventions

Project-specific rules on top of idiomatic Rust (see the `rust-best-practices` skill). These come from code review — follow them so the same notes don't recur:

- **Named status codes, not integer literals.** When building `ServerFnError::ServerError { code, .. }` in `server_fns.rs`, use named constants (e.g. `NOT_FOUND`, `FORBIDDEN`) rather than bare `404`/`403`, so error paths read at a glance.
- **Avoid `Option<bool>` parameters.** `Some(false)` is ambiguous at the call site. For a two-state flag on a server function, prefer a plainly named `bool` with an obvious default, or a small purpose-named enum.
- **Comment only what isn't obvious.** Add a comment when the code would not be clear to a senior developer reading it — the *why*, a non-obvious constraint, an invariant, a tradeoff, or a workaround (e.g. why a query is split in two, why a value clamps). Don't restate what the code already says or narrate the mechanics; if the code is self-evident, leave it uncommented. When the reason is genuinely subtle, a longer note is fine — clarity matters more than brevity.
