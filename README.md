# Horae

Self-hostable time tracking — like Harvest or Kimai, but fully yours.

Built with Rust, [Dioxus](https://dioxuslabs.com/) (fullstack SSR + WASM), PostgreSQL, and Axum.

______________________________________________________________________

## Features

| Route | Description |
|---|---|
| `/` | Dashboard — weekly hours, active projects, timer |
| `/time` | Time entries list with add/edit/delete |
| `/timesheet` | Day / Week / Calendar views with weekly totals |
| `/clients` | Client list + create (admin) |
| `/projects` | Project list + create (admin); detail with assignments |
| `/approvals` | Submit/approve/reject time submissions (manager+) |
| `/reports` | Grouped time reports with CSV/XLSX export |
| `/admin/users` | User + task management (admin) |
| `/settings` | App settings |

______________________________________________________________________

## Self-hosting

Horae ships as a NixOS module:

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

______________________________________________________________________

## Configuration

| Variable | Default | Description |
|---|---|---|
| `SESSION_SECRET` | `dev-secret-...` | Cookie signing secret — **always set in production** |
| `HORAE_HOST` | `127.0.0.1` | Bind address |
| `HORAE_PORT` | `3000` | Listen port |
| `HORAE_LOG` | `info` | Log level (`trace`, `debug`, `info`, `warn`, `error`) |
| `OIDC_ISSUER` | _(unset)_ | OIDC provider URL (required in production) |
| `OIDC_CLIENT_ID` | _(unset)_ | OIDC client ID |
| `OIDC_CLIENT_SECRET` | _(unset)_ | OIDC client secret |

______________________________________________________________________

## CLI

```sh
horae migrate run                   # apply pending migrations
horae migrate reset --confirm       # drop + re-create (dev only)
horae seed                          # insert demo data (idempotent)
horae serve --host 0.0.0.0 --port 3000
horae user list
horae user create --email admin@example.com --name "Admin" --role admin
```

______________________________________________________________________

## API

### Harvest-compatible (read-only)

Endpoints at `/harvest/v2` matching the [Harvest API v2](https://help.getharvest.com/api-v2/) shape:

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

______________________________________________________________________

## Resources

- [CONTRIBUTING.md](CONTRIBUTING.md) — development setup and contribution guidelines
- [SPEC.md](SPEC.md) — Phase 1 build specification
- [DESIGN.md](DESIGN.md) — design system and component conventions
