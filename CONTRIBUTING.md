# Contributing to Horae

Thank you for your interest in contributing! Before you start, read through
this guide to set up your environment and understand the project conventions.

---

## Getting started

**Requirements:** Nix with flakes enabled, and a running PostgreSQL instance
(the dev VM provides one).

```sh
nix develop                                      # enter the dev shell
nix run .#qemu-vm                                # boot the dev VM (PostgreSQL on :5432)
cargo run --features server -- migrate run       # apply DB migrations
cargo run --features server -- seed              # load seed data
DEV_LOGIN=1 dx serve                             # start the dev server on :8080
```

Open <http://localhost:8080/auth/login> and click **Sign in as Admin**.

> `DATABASE_URL` defaults to `postgres://localhost/horae` and is exported
> automatically by the dev shell.

---

## System setup

<details>
<summary>macOS</summary>

macOS requires [nix-darwin](https://github.com/nix-darwin/nix-darwin) for the
Linux builder, which is needed to run the e2e nixosTest (`nix flake check`)
and to cross-compile the server binary for Linux targets locally.

Enable the Linux builder in your nix-darwin configuration:

```nix
nix = {
  # Required for the Linux builder to be trusted by the Nix daemon.
  settings.trusted-users = [ "root" "@admin" ];
  linux-builder = {
    enable = true;
    ephemeral = true;
    maxJobs = 4;
    config = {
      virtualisation = {
        darwin-builder = {
          diskSize = 40 * 1024;   # 40 GiB
          memorySize = 8 * 1024;  # 8 GiB
        };
        cores = 6;
      };
    };
  };
};
```

See the [nix-darwin Linux builder docs](https://github.com/nix-darwin/nix-darwin/blob/master/modules/nix/linux-builder.nix)
for full configuration options and activation instructions.

</details>

<details>
<summary>Linux</summary>

Linux works out of the box with Nix and flakes enabled. Enable flakes in your
Nix configuration if you haven't already:

```nix
nix.settings.experimental-features = [ "nix-command" "flakes" ];
```

</details>

---

## Development workflow

```sh
nix develop                                  # enter the dev shell
nix run .#qemu-vm                            # start the dev VM (PostgreSQL)
cargo run --features server -- migrate run   # apply DB migrations
cargo run --features server -- seed          # load seed data
DEV_LOGIN=1 dx serve                         # dev server with hot reload on :8080
```

**Tests:**

```sh
cargo test -p horae-core                        # pure domain tests (no DB needed)
cargo test --features server                    # integration tests (needs Postgres with CREATEDB)
nix flake check                                 # full suite: fmt + e2e nixosTest
```

---

## Rust guidelines

### No `include!` macro for documentation

Do not use the `include!` macro (or `include_str!`) to embed documentation
from external files into doc comments or module-level docs. The macro resolves
at `rustc` compile time, which means Nix will re-hash and recompile the crate
whenever the included file changes — even though `.md` files are excluded from
the Nix source filter at the Nix evaluation level, the rustc invocation itself
sees the file as an input.

Write documentation directly in the source file instead.

### Error types must be structs

Error types must be **structs**, not enums. Model them after
[`std::io::Error`](https://doc.rust-lang.org/std/io/struct.Error.html): a
single public struct with an opaque inner kind (a private enum or a boxed
trait object). This gives you:

- An opaque public surface — callers match on methods, not variants, so you
  can evolve the internals without breaking them.
- Room to add fields (e.g. context, spans, request IDs) without a breaking
  change.
- A natural `impl std::error::Error` implementation.

```rust
// Preferred
pub struct AppError(AppErrorKind);

enum AppErrorKind {
    NotFound(String),
    Forbidden,
    // ...
}

impl AppError {
    pub fn not_found(msg: impl Into<String>) -> Self { ... }
    pub fn is_not_found(&self) -> bool { ... }
}

// Avoid
pub enum AppError {
    NotFound(String),
    Forbidden,
}
```

---

## Submitting changes

**Branch naming:** `<username>/<issue>` (e.g. `alice/42`)

If a PR is intended to close an issue, include a
[closing keyword](https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/linking-a-pull-request-to-an-issue#linking-a-pull-request-to-an-issue-using-a-keyword)
in the PR description (e.g. `Closes #42`).
