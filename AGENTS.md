# AGENTS.md — xtask-watch

Guidelines for agentic coding tools working in this repository.

---

## Repository Overview

`xtask-watch` is a Rust library crate providing a `Watch` helper that relaunches a
command when source files change. It also exposes a `WatchLock` / `WatchLockGuard` pair
so external readers (e.g. an HTTP dev-server) can coordinate with ongoing rebuilds.

```
xtask-watch/
├── src/
│   └── lib.rs          # Entire library (single-file crate)
├── examples/           # Usage examples
└── .github/workflows/  # CI definitions
```

---

## Build, Lint, and Test Commands

All commands are run from the repository root unless noted otherwise.

### Standard check/build
```bash
cargo check --workspace --all-features
cargo build --workspace --all-features
```

### Run all tests
```bash
cargo test --workspace
```

### Run a single test
```bash
# By test name (substring match)
cargo test <test_name>
```

### Formatting
```bash
# Check (CI uses this)
cargo fmt --all -- --check

# Apply
cargo fmt --all
```

### Linting (clippy)
```bash
# CI command — all warnings are errors
cargo clippy --all --tests --all-features -- -D warnings

# Local (softer, same flags)
cargo clippy --all --tests --all-features
```

### CI matrix
CI runs on stable Rust and the MSRV declared in `Cargo.toml` (`rust-version`). Always
verify `cargo fmt` and `cargo clippy` pass before committing.

Current MSRV: **1.85.1** (Rust edition 2024). Do not use features introduced after this
version without updating `rust-version` in `Cargo.toml`.

---

## Changelog — MANDATORY

**Every pull request or commit that contains a user-visible change MUST add an entry to
`CHANGELOG.md` under the `## [Unreleased]` section.** This is not optional.

### What counts as user-visible

- New public types, functions, methods, or trait impls
- Changed behaviour of existing public API
- Bug fixes observable by users
- Dependency version bumps that affect the public API or MSRV
- Removed or renamed public items

### What does NOT need a changelog entry

- Internal refactors with no observable behaviour change
- CI / tooling changes
- Documentation-only changes (though you may add a `### Documentation` entry if useful)

### Format

Follow the [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format already
in use. Add a subsection under `## [Unreleased]` using one of:

```markdown
## [Unreleased]

### Added
- Short description of the new thing (#PR).

### Changed
- Short description of what changed and why (#PR).

### Fixed
- Short description of the bug that was fixed (#PR).

### Removed
- Short description of what was removed (#PR).
```

Use the imperative mood ("Add …", "Fix …", "Remove …"). Reference the PR or issue
number in parentheses when one exists.

### When to write the entry

Write the changelog entry **in the same commit as the code change**, not afterward.
If you forget, the release will go out with an empty `[Unreleased]` section and the
change will be invisible to users of the crate.

---

## Code Style Guidelines

### Formatting

- Use `cargo fmt` (default `rustfmt` settings — no `rustfmt.toml` in this repo).
- 4-space indentation; no tabs.
- Trailing commas in multi-line struct literals and function call arguments.

### Imports

- Group imports using nested paths where possible:
  ```rust
  use std::{path::PathBuf, process::Command, sync::Arc};
  ```
- Standard library imports first, then external crates, then `crate::` / `super::`.

### Naming Conventions

| Item | Convention | Example |
|------|-----------|---------|
| Types / traits | `PascalCase` | `Watch`, `WatchLock` |
| Functions / methods | `snake_case` | `run_command`, `handle_event` |
| Fields / variables | `snake_case` | `watch_paths`, `debounce` |
| Constants / statics | `SCREAMING_SNAKE_CASE` | `METADATA` |
| Modules | `snake_case`, match filename | `lib` |

### Error Handling

- All fallible functions return `anyhow::Result<T>`.
- Prefer `.context("…")` / `.with_context(|| …)` to annotate errors.
- `unwrap()` is acceptable only when the invariant is guaranteed by surrounding logic.
- `expect("message")` is acceptable where a panic would indicate a programmer bug.
- Do **not** introduce custom error types — stay with `anyhow` throughout.

### Types and Traits

- Public API structs are marked `#[non_exhaustive]` to preserve semver compatibility.
- Public API structs that are CLI entry points derive `clap::Parser`.
- Builder pattern: methods take `mut self` and return `Self` for chaining.

### Documentation

- `#![deny(missing_docs)]` is active in `lib.rs` — **every public item must have a doc
  comment**.
- Use `///` for item-level docs; `//!` for module/crate-level docs.
- Include `# Examples` sections (with ` ```rust,no_run ``` `) for significant public API.

### Logging

- Use the `log` crate macros: `log::trace!`, `log::debug!`, `log::info!`, `log::warn!`,
  `log::error!`.
- Do not use `println!` / `eprintln!` for diagnostic output.

---

## Dependency Philosophy

- Keep dependencies minimal.
- `anyhow`, `clap`, `cargo_metadata`, and `camino` are re-exported for downstream crates
  (e.g. `xtask-wasm`) — preserve these re-exports.
- `glob` and `lazy_static` are internal implementation details; do not re-export them.
- `libc` is a Unix-only dependency (`[target.'cfg(unix)'.dependencies]`); guard any
  libc usage with `#[cfg(unix)]`.

---

## Platform-Specific Code

- Unix-only code (e.g. `SIGTERM` signalling) must be wrapped in `#[cfg(unix)]` blocks.
- Windows must still compile and behave correctly — fall back gracefully where POSIX
  primitives are unavailable.

---

## MSRV Policy

The minimum supported Rust version is declared in `Cargo.toml` under `rust-version`. CI
verifies both stable and MSRV. Do not use language or library features introduced after
that version without updating `rust-version` **and** adding a `### Changed` entry to the
changelog.
