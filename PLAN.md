# Horae — Implementation Plan

## Context

Horae is a new self-hostable time tracking web application (similar to Harvest, Kimai, Invoice Ninja) written entirely in Rust. The project is a blank slate — only a "Hello, world!" `main.rs` and a stub `flake.nix` exist. The frontend is Dioxus (Rust → WASM) with subsecond hot-patching via `dx serve --hotpatch` for fast UI iteration. Axum is the HTTP server (Dioxus fullstack uses it internally). Clap handles the CLI.

---

## Phase 1: Nix Setup

### `flake.nix`

Add inputs:
```
treefmt-nix.url = "github:numtide/treefmt-nix"
  inputs.nixpkgs.follows = "nixpkgs"
fenix.url = "github:nix-community/fenix"
  inputs.nixpkgs.follows = "nixpkgs"
  inputs.rust-analyzer-src.follows = ""
```

Outputs to produce:
- `packages.${system}.default` — Horae server binary via `rustPlatform.buildRustPackage` with `--features server` using fenix toolchain
- `devShells.${system}.default` — fenix toolchain + dioxus-cli + sqlx-cli + wasm-pack + nil (Nix LSP)
- `formatter.${system}` — treefmt with nixpkgs-fmt, rustfmt, taplo, mdformat
- `checks.${system}.fmt` — treefmt check
- `nixosModules.default` — server module (see below)
- `nixosConfigurations.default` — test VM inline in flake

Use `nixpkgs.lib.genAttrs lib.systems.flakeExposed` as `forAllSystems` (no flake-utils needed).

### `nixos/modules/horae/default.nix`

Options under `services.horae`:

| Option | Type | Default |
|---|---|---|
| `enable` | bool | false |
| `package` | package | self packages default |
| `host` | str | "127.0.0.1" |
| `port` | port | 3000 |
| `database.url` | nullOr str | null |
| `database.createLocally` | bool | true |
| `dataDir` | path | "/var/lib/horae" |
| `secretKeyFile` | nullOr path | null |
| `logLevel` | str | "info" |
| `openFirewall` | bool | false |

`systemd.services.horae` config:
- `ExecStartPre = horae migrate`
- `ExecStart = horae serve --host ... --port ...`
- `DynamicUser = true`, `StateDirectory = "horae"`, standard hardening flags

### `nixosConfigurations.default`

Minimal x86_64-linux VM with `services.horae.enable = true` and `createLocally = true` for `nix run .#vm` dev use.

---

## Phase 2: Rust Skeleton

### `Cargo.toml` dependencies + features

```toml
[features]
default = ["server"]
server = ["dioxus/server", "dep:tokio", "dep:axum", "dep:tower-http", "dep:sqlx", "dep:argon2", "dep:tower-sessions"]
web = ["dioxus/web"]

[dependencies]
dioxus = { version = "0.7", features = ["fullstack", "router"] }
clap = { version = "4", features = ["derive", "env"] }
serde = { version = "1", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v7", "serde"] }
extism = { version = "1", optional = true }  # server feature; WASM plugin host
thiserror = "2"
anyhow = "1"
tracing = "0.1"

# server-only (optional, enabled by "server" feature)
tokio = { version = "1", features = ["full"], optional = true }
axum = { version = "0.7", features = ["macros", "multipart", "ws"], optional = true }
tower-http = { version = "0.6", features = ["fs", "compression-gzip", "trace"], optional = true }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "postgres", "uuid", "chrono", "migrate"], optional = true }
argon2 = { version = "0.5", optional = true }
tower-sessions = { version = "0.14", features = ["sqlite-store"], optional = true }
tracing-subscriber = { version = "0.3", features = ["env-filter"], optional = true }
```

`dx serve` builds both the `web` (WASM) and `server` targets simultaneously. The Nix package uses `--features server` only (builds the server binary for deployment).

### `Dioxus.toml`

Required config for the dx CLI. Declares the app name, asset directory, web index template, and platform targets.

### Directory structure

```
horae/
├── Cargo.toml / Cargo.lock
├── Dioxus.toml              ← dx CLI config
├── flake.nix / flake.lock
├── DESIGN.md                ← design language doc
├── CLAUDE.md -> DESIGN.md   ← symlink
├── migrations/              ← sqlx migrations (0001–0006)
├── assets/                  ← static files served by Dioxus (CSS, icons, fonts)
│   └── css/horae.css
├── nixos/modules/horae/default.nix
└── src/
    ├── main.rs              ← cfg-gated entry points (server vs web)
    ├── cli.rs               ← clap Cli structs (server feature only)
    ├── config.rs            ← AppConfig (server feature only)
    ├── app.rs               ← root Dioxus component + Router<Route>
    ├── route.rs             ← Route enum (Routable derive)
    ├── error.rs             ← AppError
    ├── db.rs                ← pool init + sqlx::migrate!() (server feature)
    ├── state.rs             ← server state passed via Dioxus context
    ├── models.rs            ← re-exports; submodules in src/models/
    ├── models/              ← user.rs, client.rs, project.rs, task.rs, time_entry.rs, invoice.rs
    ├── pages.rs             ← re-exports; submodules in src/pages/
    ├── pages/               ← dashboard.rs, auth.rs, clients.rs, projects.rs, time.rs, invoices.rs, admin.rs
    ├── components.rs        ← re-exports; submodules in src/components/
    ├── components/          ← nav.rs, sidebar.rs, timer_widget.rs, table.rs, form.rs, badge.rs
    ├── plugin.rs            ← extism host fn definitions + re-exports
    ├── plugin/              ← registry.rs, host.rs, event.rs, manifest.rs
    └── server.rs            ← server functions (#[server] macros) — server feature only
```

### `src/main.rs` structure

```rust
#[cfg(feature = "server")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // clap dispatch: migrate or serve
}

#[cfg(feature = "web")]
fn main() {
    dioxus::launch(app::App);
}
```

The `serve` subcommand calls `dioxus::launch` (or `dioxus::serve` for custom axum router extensions) with the root `App` component.

---

## Phase 3: Data Model & API

### Core entities (all use UUID v7 PK + created_at/updated_at)

- **users**: id, email, display_name, password_hash (argon2id), role, is_active
- **clients**: id, name, email, currency, created_by
- **projects**: id, client_id, name, code, budget_hours, billing_method, hourly_rate, is_active
- **tasks**: id, project_id, name, hourly_rate, is_billable
- **time_entries**: id, user_id, project_id, task_id, started_at, ended_at (NULL = running), duration_seconds, notes, is_billable, invoice_id
- **invoices**: id, client_id, invoice_number, status (draft/sent/paid/void), issued_date, due_date, total_amount

### CLI subcommands (`clap`, server feature only)

```
horae serve           -- start HTTP server (calls dioxus::launch)
horae migrate         -- run pending migrations
horae migrate reset   -- drop + recreate (dev, --confirm required)
horae user create     -- create admin user
horae user list       -- list users
```

### Routing (dioxus-router `Routable` derive)

```rust
#[derive(Routable, Clone)]
pub enum Route {
    #[route("/auth/login")]     Login {},
    #[route("/auth/register")]  Register {},
    #[layout(AppLayout)]
    #[route("/")]               Dashboard {},
    #[route("/clients")]        ClientList {},
    #[route("/clients/:id")]    ClientDetail { id: Uuid },
    #[route("/projects")]       ProjectList {},
    #[route("/projects/:id")]   ProjectDetail { id: Uuid },
    #[route("/time")]           TimeList {},
    #[route("/invoices")]       InvoiceList {},
    #[route("/invoices/:id")]   InvoiceDetail { id: Uuid },
    #[route("/admin/users")]    AdminUsers {},
    #[route("/settings")]       Settings {},
    #[not_found]                NotFound {},
}
```

### Server functions (`src/server.rs`)

All data fetching and mutations use `#[server]` macros — Dioxus automatically registers them as HTTP endpoints on the axum router. Examples:

```rust
#[server] async fn list_time_entries(...) -> Result<Vec<TimeEntry>, ServerFnError>
#[server] async fn start_timer(project_id: Uuid) -> Result<TimeEntry, ServerFnError>
#[server] async fn stop_timer(entry_id: Uuid) -> Result<TimeEntry, ServerFnError>
#[server] async fn login(email: String, password: String) -> Result<(), ServerFnError>
```

No manual axum route wiring needed for data endpoints — server functions handle it.

---

## Plugin System

Plugins are **WASM modules loaded at runtime via [extism](https://github.com/extism/extism)**. Any language that compiles to WASM (Rust, Go, TypeScript, C, Zig, etc.) can write a Horae plugin. Plugins run sandboxed — they can only call host functions that Horae explicitly exposes.

### `src/plugin/` module

```
src/plugin.rs          ← re-exports + host function definitions
src/plugin/
    registry.rs        ← PluginRegistry: scans plugins/ dir, loads *.wasm at startup
    host.rs            ← extism host functions (db queries, logging, HTTP requests)
    event.rs           ← AppEvent enum (serialized to JSON and passed to plugins)
    manifest.rs        ← plugin.toml schema (name, version, hooks subscribed to)
```

### Plugin interface

Each plugin declares in a `plugin.toml` sidecar (or embedded section) which events it handles:
```toml
[plugin]
name = "slack-notifier"
version = "1.0.0"
hooks = ["time_entry_created", "invoice_sent"]
```

Horae calls the plugin's exported function matching each hook name, passing a JSON payload. Plugins return a JSON response (or nothing).

### Host functions exposed to plugins

Via extism's `host_fn!` macro:
- `horae_log(level, message)` — structured logging
- `horae_db_query(sql, params_json)` → JSON rows (read-only)
- `horae_http_post(url, body_json)` → response (for webhooks, outbound integrations)
- `horae_config_get(key)` → plugin's own config value from settings

### `PluginRegistry` in `AppState`

`AppState` gains `plugins: Arc<PluginRegistry>`. At startup, the registry scans the `plugins/` data directory, loads each `.wasm` file via `extism::Plugin::new()`, and stores a handle per plugin. On events, `registry.dispatch(event)` calls all plugins subscribed to that hook concurrently.

### Hook call sites

- `server.rs` (server functions) — dispatch `TimeEntryCreated` / `TimeEntryStopped` after DB writes
- `server.rs` — dispatch `InvoiceCreated` / `InvoiceSent` after invoice mutations
- `server.rs` — dispatch `UserLoggedIn` for audit log plugins

### UI extension

WASM plugins cannot render Dioxus components directly. Instead, plugins can return a `DashboardWidget` JSON spec (title, body HTML/markdown) which Horae renders in designated plugin slots on the dashboard and settings pages.

### Adding/removing plugins

Plugins are dropped into `{dataDir}/plugins/` alongside a `plugin.toml`. A future admin UI page will list loaded plugins and allow enable/disable without restart (hot-reload via `extism::Plugin` re-instantiation).

---

## Design Language (`DESIGN.md` + `CLAUDE.md` symlink)

`DESIGN.md` covers:
1. Color palette via CSS custom properties (Gitea-inspired teal/green primary, neutral grays)
2. Typography: system font stack only
3. Component inventory: nav bar, sidebar, data tables, forms, status badges, timer widget
4. Interaction principles: Dioxus reactivity for all UI updates, no JS frameworks
5. Component conventions: one component per file, `snake_case` props
6. Accessibility: keyboard nav, ARIA labels, WCAG AA contrast

`CLAUDE.md` is a symlink to `DESIGN.md` so it's picked up as project instructions.

---

## Development Workflow

- **Hot reload**: `dx serve --hotpatch` — RSX template changes are instant, Rust logic changes hot-patch without restart
- **Standard dev**: `dx serve` (without `--hotpatch`) for stable iteration
- **DB migrations**: `cargo run --features server -- migrate` (or `sqlx migrate run` directly)
- **Production build**: `dx build --release` then `nix build`

---

## Phased Delivery

| Phase | Deliverable | Done when |
|---|---|---|
| 1 | `flake.nix` fully populated | `nix flake check` passes |
| 1 | `nixos/modules/horae/default.nix` | Module options compile |
| 2 | `Cargo.toml` + `Dioxus.toml` + src skeleton | `dx serve` renders placeholder page |
| 3a | Auth + time entries MVP | Login, start/stop timer, list entries |
| 3b | Clients, projects, invoices | Full CRUD, invoice detail |
| 3c | Polish | Invoice PDF, pagination, CSV export, admin UI |

---

## Files to Create / Modify

- `flake.nix` (modify)
- `Cargo.toml` (modify)
- `Dioxus.toml` (create)
- `src/main.rs` (rewrite)
- `src/cli.rs`, `src/config.rs`, `src/app.rs`, `src/route.rs`, `src/error.rs`, `src/db.rs`, `src/state.rs`, `src/server.rs` (create)
- `src/models.rs` + `src/models/*.rs` (create)
- `src/pages.rs` + `src/pages/*.rs` (create)
- `src/components.rs` + `src/components/*.rs` (create)
- `src/plugin.rs` + `src/plugin/*.rs` (create)
- `migrations/0001–0006.sql` (create)
- `assets/css/horae.css` (create)
- `nixos/modules/horae/default.nix` (create)
- `DESIGN.md` (create), `CLAUDE.md -> DESIGN.md` (symlink)

## Verification

1. `nix flake check` — formatter + build checks pass
2. `dx serve` — browser shows placeholder app with hot reload working
3. `dx serve --hotpatch` — UI changes reflect without restart
4. `cargo test --features server` — unit tests on model/server functions pass
5. Manual: register user, start timer, stop timer, create invoice
6. `nix build` — Nix package builds cleanly with `--features server`
