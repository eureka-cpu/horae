# CLI Contract

The `horae` binary is the server-and-tooling entry point. It is built from the
`server` feature of the single Horae crate; the CLI is defined in `src/cli.rs`
(clap) and dispatched in `src/main.rs`. Configuration is read from the
environment in `src/config.rs`.

Invocation forms:

1. `horae <subcommand> [args]` when running the compiled server binary.
1. `cargo run --features server -- <subcommand> [args]` during development.
1. Running the binary with **no subcommand** defaults to `serve` (with default
   arguments). This exists so `dx serve` can launch the binary bare.

## Subcommands

### `serve`

Starts the HTTP server: the Dioxus fullstack application plus the layered Axum
routes (`/health`, CSV/XLSX export, auth router, and the read-only Harvest v2
API), all under a Postgres-backed session layer.

Arguments:

1. `--host <HOST>` — bind address. Defaults to `127.0.0.1`. Also settable via
   the `HORAE_HOST` environment variable.
1. `--port <PORT>` — listen port. Defaults to `3000`. Also settable via the
   `HORAE_PORT` environment variable.

Behavior notes:

1. On startup, `serve` creates the Postgres pool and **runs migrations eagerly**
   before serving, then initializes shared application state. So a fresh `serve`
   applies pending migrations without a separate `migrate run`.
1. When running under `dx serve`, the `IP` and `PORT` environment variables (set
   by the Dioxus dev tooling for hot-reload proxying) **override** the resolved
   `--host` / `--port` values.

### `migrate run`

Applies all pending database migrations, then exits. This is `migrate` with the
default `run` action.

### `migrate reset --confirm`

Intended to drop and recreate the database (development only).

1. `--confirm` is **required**. Without it the command prints
   `Pass --confirm to reset the database.` and exits with a non-zero status
   (exit code `1`).
1. **Current behavior gap**: with `--confirm`, the implementation only runs
   migrations (the same as `migrate run`); it does **not** actually drop and
   recreate the database yet. The destructive drop/recreate is still to be
   implemented.

### `seed`

Populates the database with demo data and then verifies it. Documented as safe
to re-run (idempotent). Creates, among other data, the admin user used by
`DEV_LOGIN`.

### `user create --email <EMAIL> --name <NAME> [--role <ROLE>]`

Intended to create a user.

1. `--email` — required.
1. `--name` — required.
1. `--role` — optional; defaults to `user`.
1. **Status: stub / not implemented.** The handler logs the intended action but
   performs no database write (a `TODO` notes it should insert a user via an
   OIDC subject or admin invite). Running it has no persistent effect today.

### `user list`

Intended to list all users.

1. **Status: stub / not implemented.** The handler prints `No users found.`
   unconditionally (a `TODO` notes it should query and print users). It does not
   read the database.

## Environment Variables

Defaults and behavior are taken from `src/config.rs`, `.env.example`, and the
README. The `serve` command's bind address can also come from `--host` /
`--port` flags, which the environment variables back.

### Read by `AppConfig` (`src/config.rs`)

1. `DATABASE_URL` — Postgres connection string. Default in code:
   `postgres://localhost/horae` (the `.env.example` sample uses
   `postgres://localhost:5432/horae`). Used by every subcommand that touches the
   database.
1. `HORAE_HOST` — server bind address. Default `127.0.0.1`. Backs `serve --host`.
1. `HORAE_PORT` — server listen port. Default `3000`. Backs `serve --port`.
1. `HORAE_LOG` — log level: `trace`, `debug`, `info`, `warn`, or `error`.
   Default `info`. (The standard `RUST_LOG` env filter, when set, takes
   precedence over this value.)
1. `DEV_LOGIN` — when `1` or `true`, skip OIDC and enable one-click login as the
   seeded admin user. Default: disabled (any other value, or unset). For
   development only.
1. `SESSION_SECRET` — secret used to sign session cookies. Default in code:
   `dev-secret-change-me-in-production` (the `.env.example` sample uses
   `changeme`). **Always set a strong value in production.**

### Read by the OIDC auth layer (`src/auth/oidc.rs`), not `AppConfig`

These are consumed directly by the OIDC module rather than parsed into
`AppConfig`. They are required for production (non-`DEV_LOGIN`) authentication;
all default to unset.

1. `OIDC_ISSUER` — OIDC provider URL used to discover provider metadata.
1. `OIDC_CLIENT_ID` — OIDC client id.
1. `OIDC_CLIENT_SECRET` — OIDC client secret.

## Exit Behavior Summary

1. `serve` runs until the process is terminated; it applies migrations before
   listening.
1. `migrate reset` without `--confirm` exits with code `1` and a message.
1. Other subcommands run to completion and exit; database errors surface as
   process errors (non-zero exit) via the `anyhow` result in `main`.
