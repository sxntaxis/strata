# Strata

Strata is a Rust time tracker with a terminal UI and a small CLI.

## Goals

- Keep the behavior stable and predictable.
- Keep the codebase easy to read and review.
- Keep domain, storage, and UI concerns separated.

## Build And Run

```bash
cargo run
```

Run CLI commands with arguments:

```bash
cargo run -- report --today
```

## Architecture

- `src/domain.rs`: business rules (categories, sessions, day boundary, reports).
- `src/storage.rs`: persistence (CSV/JSON, paths, atomic writes, backups).
- `src/app.rs` + `src/app/*`: TUI orchestration, rendering, and key handling.
- `src/cli.rs`: command handling and output formatting for non-TUI usage.
- `src/sand/*`: sand simulation and rendering primitives.

When changing code, keep these boundaries strict:

- Domain does not perform file I/O.
- Storage does not contain UI behavior.
- UI orchestrates, but avoids embedding core business logic.

## Data Locations

Strata stores runtime data in XDG paths:

- Data: `~/.local/share/strata/`
- State: `~/.local/state/strata/`

Repo-local runtime artifacts are intentionally ignored by git.

## Quality Gates

Before opening a PR, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```
