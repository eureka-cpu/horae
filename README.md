# Horae

Self-hostable time tracking — like Harvest or Kimai, but fully yours.

Built with Rust, [Dioxus](https://dioxuslabs.com/) (fullstack SSR + WASM), PostgreSQL, and Axum.

---

## Quick start (dev)

### Prerequisites

- [Nix](https://nixos.org/download/) with flakes enabled
- A nix-darwin Linux builder (for the dev VM) — or a running PostgreSQL instance

### 1. Start the dev VM (provides PostgreSQL)

```sh
nix develop                    # enter the dev shell
nix run .#dev-vm               # boots a NixOS VM with Postgres (QEMU + HVF on Apple Silicon)
```

The VM forwards ports 2222 (SSH), 3000, and 5432 to localhost. Log in via:

```sh
ssh -o StrictHostKeyChecking=no -p 2222 root@127.0.0.1
```

> **Note:** QEMU's port 5432 forwarding can be unreliable. Use an SSH tunnel instead:
> ```sh
> ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
>     -N -L 15432:/run/postgresql/.s.PGSQL.5432 -p 2222 root@127.0.0.1
> ```
> Then use `DATABASE_URL=postgres://horae@127.0.0.1:15432/horae` for all commands below.

### 2. Run migrations and seed

```sh
DATABASE_URL=postgres://horae@127.0.0.1:15432/horae cargo run --features server -- migrate run
DATABASE_URL=postgres://horae@127.0.0.1:15432/horae cargo run --features server -- seed
```

The seed creates: 1 org, 1 admin user, 2 clients, 2 projects, 4 tasks, 10 time entries.

### 3. Start the dev server

```sh
DEV_LOGIN=1 DATABASE_URL=postgres://horae@127.0.0.1:15432/horae dx serve
```

Open **http://localhost:8080/auth/login** and click **"Sign in as Admin"**.

The first build compiles both the server binary and WASM bundle (~30s). Subsequent hot-reloads are fast.

---

## Using without the dev VM

If you have PostgreSQL running locally:

```sh
createdb horae
DATABASE_URL=postgres://localhost/horae cargo run --features server -- migrate run
DATABASE_URL=postgres://localhost/horae cargo run --features server -- seed
DEV_LOGIN=1 DATABASE_URL=postgres://localhost/horae dx serve
```

---

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `postgres://localhost/horae` | PostgreSQL connection string |
| `DEV_LOGIN` | _(unset)_ | Set to `1` to enable one-click admin login (no OIDC) |
| `SESSION_SECRET` | `dev-secret-...` | Cookie signing secret (change in prod) |
| `HORAE_HOST` | `127.0.0.1` | Bind address |
| `HORAE_PORT` | `3000` | Port (overridden to `8080` by `dx serve`) |
| `HORAE_LOG` | `info` | Log level (`trace`, `debug`, `info`, `warn`, `error`) |
| `OIDC_ISSUER` | _(unset)_ | OIDC provider URL (required in prod) |
| `OIDC_CLIENT_ID` | _(unset)_ | OIDC client ID |
| `OIDC_CLIENT_SECRET` | _(unset)_ | OIDC client secret |

---

## CLI commands

```sh
horae migrate run                   # apply pending migrations
horae migrate reset --confirm       # drop + re-create (dev only)
horae seed                          # insert demo data (idempotent)
horae serve --host 0.0.0.0 --port 3000
horae user list
horae user create --email admin@example.com --name "Admin" --role admin
```

---

## Pages

| Route | Description |
|---|---|
| `/auth/login` | Sign-in page (Axum-served, DEV_LOGIN or OIDC) |
| `/` | Dashboard — weekly hours, active projects, timer |
| `/time` | Time entries list with add/edit/delete |
| `/timesheet` | Day / Week / Calendar views with weekly totals |
| `/clients` | Client list + create (admin) |
| `/projects` | Project list + create (admin); detail with assignments |
| `/invoices` | Invoice list (Phase 4) |
| `/approvals` | Submit/approve/reject time submissions (manager+) |
| `/reports` | Grouped time reports with CSV/XLSX export |
| `/admin/users` | User + task management (admin) |
| `/settings` | App settings |

## API

### Internal server functions

All data mutations use Dioxus `#[server]` functions (auto-routed, session-authenticated).

### Harvest-compatible API

Read-only endpoints at `/harvest/v2` matching the [Harvest API v2](https://help.getharvest.com/api-v2/) shape:

```
GET /harvest/v2/users/me
GET /harvest/v2/time_entries[?from=&to=&user_id=&project_id=&is_running=&page=&per_page=]
GET /harvest/v2/time_entries/{id}
GET /harvest/v2/projects[/{id}]
GET /harvest/v2/clients[/{id}]
GET /harvest/v2/tasks[/{id}]
GET /harvest/v2/users
```

Auth is session-based (cookie). Bearer token auth is planned for Phase 2.

### Export

```
GET /api/reports/export/csv?from=YYYY-MM-DD&to=YYYY-MM-DD
GET /api/reports/export/xlsx?from=YYYY-MM-DD&to=YYYY-MM-DD
```

---

## NixOS deployment

```nix
{
  imports = [ horae.nixosModules.default ];

  services.horae = {
    enable = true;
    host = "127.0.0.1";
    port = 3000;
    database.createLocally = true;   # manages local PostgreSQL
    # secretKeyFile = /run/secrets/horae-env;
    # openFirewall = true;
  };
}
```

---

## Project layout

```
horae/
├── core/                # Pure domain logic (duration, rounding, money, state machine)
├── src/
│   ├── main.rs          # CLI entry + custom Axum server
│   ├── app.rs           # Root Dioxus component
│   ├── route.rs         # SPA routes (Routable derive)
│   ├── server_fns.rs    # #[server] functions (CRUD, auth, reports)
│   ├── auth/            # Session management, DEV_LOGIN, OIDC stub
│   ├── harvest/         # Harvest-compatible API (/harvest/v2)
│   ├── reports.rs       # CSV/XLSX export Axum handlers
│   ├── models/          # Domain types (User, Project, TimeEntry, ...)
│   ├── pages/           # Page components (dashboard, time, timesheet, ...)
│   ├── components/      # Shared UI (nav, sidebar, timer, table, form, badge)
│   ├── seed.rs          # Demo data seeder
│   ├── db.rs            # PgPool + migrations
│   ├── config.rs        # Environment config
│   └── state.rs         # Global AppState (OnceCell)
├── migrations/          # sqlx SQL migration (0001_init.sql)
├── assets/css/          # Design system CSS
├── nixos/modules/horae/ # NixOS module
├── flake.nix            # Nix flake (dev shell, VM, package)
├── SPEC.md              # Phase 1 build spec
└── Dioxus.toml          # dx CLI configuration
```
