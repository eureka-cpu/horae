# Contributing to Horae

Thank you for your interest in contributing! Before you start, read through
this guide to set up your environment and understand the project conventions.

---

## Getting started

See the [README](README.md) for a full quick-start walkthrough: starting the
dev VM, running migrations, and launching the dev server.

---

## System setup

<details>
<summary>macOS</summary>

macOS requires [nix-darwin](https://github.com/LnL7/nix-darwin) to get the
most out of the development environment. Specifically:

- The **Linux builder** (provided by nix-darwin) is required to run the e2e
  nixosTest (`nix flake check`) and to cross-compile the server binary for
  Linux targets locally.
- Without it, `nix flake check` will fail when it tries to build or run the
  NixOS VM test.

**Install nix-darwin:**
Follow the [nix-darwin installation guide](https://github.com/LnL7/nix-darwin?tab=readme-ov-file#installation).
Once installed, enable the Linux builder in your nix-darwin configuration:

```nix
nix.linux-builder.enable = true;
```

Then rebuild: `darwin-rebuild switch`.

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
nix develop                                          # enter the dev shell
nix run .#qemu-vm                                    # start the dev VM (PostgreSQL)
DATABASE_URL=postgres://horae@127.0.0.1:5432/horae cargo run --features server -- migrate run
DATABASE_URL=postgres://horae@127.0.0.1:5432/horae cargo run --features server -- seed
DEV_LOGIN=1 DATABASE_URL=postgres://horae@127.0.0.1:5432/horae dx serve
```

Open <http://localhost:8080/auth/login> and click **Sign in as Admin**.

**Tests:**

```sh
cargo test -p horae-core                        # pure domain tests (no DB needed)
DATABASE_URL=… cargo test --features server     # integration tests (needs Postgres with CREATEDB)
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

> The existing `AppError` enum in `src/error.rs` predates this guideline
> and will be migrated in a follow-up.

---

## Submitting changes

**Branch naming:** `feat/<topic>`, `fix/<topic>`, `chore/<topic>`

**PR checklist:**

- [ ] `cargo clippy --features server` — no warnings
- [ ] `nix fmt` — formatter passes (`treefmt`: rustfmt, taplo, nixpkgs-fmt,
      mdformat)
- [ ] Tests pass: `cargo test -p horae-core` and
      `DATABASE_URL=… cargo test --features server`
- [ ] `nix flake check` passes (requires Linux builder on macOS)
