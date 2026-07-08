# Horae

Self-hostable time tracking — like Harvest or Kimai, but fully yours.

Built with Rust, [Dioxus](https://dioxuslabs.com/) (server + WASM), SQLite, and Axum.

---

## Quick start

### With Nix (recommended)

```sh
nix develop          # enter the dev shell (rustc, cargo, dx, sqlx-cli, wasm-pack, nil)
dx serve             # hot-reload dev server on http://127.0.0.1:8080
```

### Without Nix

Install the prerequisites manually:

| Tool | Install |
|---|---|
| Rust (stable) | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| `wasm32` target | `rustup target add wasm32-unknown-unknown` |
| dioxus CLI | `cargo install dioxus-cli` |
| sqlx CLI | `cargo install sqlx-cli --no-default-features --features sqlite` |

Then:

```sh
dx serve             # dev server with hot reload on http://127.0.0.1:8080
```

---

## Development

### Hot-reload dev server

```sh
dx serve             # reload on Rust + RSX changes
dx serve --hotpatch  # subsecond hot-patch for RSX template changes (requires nightly)
```

The first run downloads all Cargo dependencies and compiles both the server binary and the WASM bundle — this takes a few minutes. Subsequent reloads are fast.

By default the server listens on `http://127.0.0.1:8080`. The database is created at `horae.db` in the current directory.

### Environment variables

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `sqlite:horae.db` | SQLite database path |
| `HORAE_HOST` | `127.0.0.1` | Bind address |
| `HORAE_PORT` | `3000` | Port (overridden to `8080` by `dx serve`) |
| `HORAE_LOG` | `info` | Log level (`trace`, `debug`, `info`, `warn`, `error`) |
| `HORAE_DATA_DIR` | `.` | Data directory (plugins live in `$HORAE_DATA_DIR/plugins/`) |

### Running without the dx CLI

```sh
# Build the server binary only (no WASM)
cargo build --features server

# Run migrations then start the server
cargo run --features server -- migrate
cargo run --features server -- serve --host 127.0.0.1 --port 3000
```

### Running migrations manually

```sh
cargo run --features server -- migrate        # apply pending migrations
cargo run --features server -- migrate reset --confirm   # drop + re-create (dev only)

# Or via sqlx-cli directly:
sqlx migrate run --database-url sqlite:horae.db
```

### User management

```sh
cargo run --features server -- user list
cargo run --features server -- user create --email admin@example.com --name "Admin" --role admin
```

---

## Production build

```sh
# 1. Build the WASM frontend + server binary
dx build --release

# 2. Or build via Nix (reproducible, self-contained)
nix build
result/bin/horae migrate
result/bin/horae serve
```

The Nix build produces a single binary at `result/bin/horae` with the WASM assets embedded.

---

## NixOS deployment

```nix
# In your NixOS configuration:
{
  imports = [ horae.nixosModules.default ];

  services.horae = {
    enable = true;
    host   = "127.0.0.1";
    port   = 3000;
    # database.createLocally = true;  # default — uses sqlite:$dataDir/horae.db
    # secretKeyFile = /run/secrets/horae-env;  # optional env file with secrets
    # openFirewall = true;  # open port in firewall
  };
}
```

Run the test VM locally:

```sh
nix run .#nixosConfigurations.default.config.system.build.vm
```

---

## Project layout

```
horae/
├── src/
│   ├── main.rs          # CLI entry points (server + web)
│   ├── app.rs           # Root Dioxus component
│   ├── route.rs         # All routes (Routable derive)
│   ├── server_fns.rs    # #[server] functions (HTTP endpoints)
│   ├── models/          # Domain types (User, Project, TimeEntry, …)
│   ├── pages/           # Page components
│   ├── components/      # Shared UI components
│   ├── plugin/          # WASM plugin host (extism)
│   ├── db.rs            # Pool + migrations
│   └── state.rs         # Global AppState (OnceCell)
├── migrations/          # sqlx SQL migrations (0001–0006)
├── assets/css/          # Design system CSS
├── nixos/modules/horae/ # NixOS module
├── DESIGN.md            # Design language reference (symlinked as CLAUDE.md)
└── Dioxus.toml          # dx CLI configuration
```

---

## Plugins

Drop a `.wasm` file and a matching `plugin.toml` into `$HORAE_DATA_DIR/plugins/`:

```toml
# plugins/my-plugin.toml
[plugin]
name    = "my-plugin"
version = "1.0.0"
hooks   = ["time_entry_created", "invoice_sent"]
```

Horae loads plugins at startup and calls the exported function matching each hook name with a JSON payload. Plugins are sandboxed via [extism](https://github.com/extism/extism) and can only call explicitly exposed host functions (`horae_log`, `horae_db_query`, `horae_http_post`, `horae_config_get`).

---

## Status

| Phase | What | Status |
|---|---|---|
| 1 | Nix flake + NixOS module | ✅ done |
| 2 | Rust skeleton — compiles, routes render | ✅ done |
| 3a | Auth (login/register) + time entry CRUD | 🔲 next |
| 3b | Clients, projects, invoices | 🔲 planned |
| 3c | Invoice PDF, pagination, CSV export, admin UI | 🔲 planned |
