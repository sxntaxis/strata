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
- Config: `~/.config/strata/`

You can override the data directory with `STRATA_DATA_DIR=/your/path`.

Repo-local runtime artifacts are intentionally ignored by git.

## Keybindings

- Open the Command Atlas in TUI with `?` or `F1`.
- In layer text entry, `?` stays a normal character; use `F1` there.
- Optional config file: `~/.config/strata/keymap.json`

Example:

```json
{
  "keymap_inherit": true,
  "time_log_path": "/home/user/.local/share/strata/time_log.csv",
  "day_start_mode": "sunrise",
  "day_start_hour": 6,
  "day_start_minute": 0,
  "first_day_of_week": "monday",
  "unbind_actions": ["open_layer_popup"],
  "keymap": {
    "f": "open_karma_popup",
    "k": null,
    "ctrl-q": "quit"
  }
}
```

Notes:

- `keymap_inherit: true` starts from built-in defaults, then applies overrides.
- Setting a key to `null` unbinds that key.
- `unbind_actions` disables specific actions by name.
- Setting `keymap_inherit: false` starts from an empty keymap.
- `time_log_path` accepts a CSV path or directory (directory auto-appends `time_log.csv`).
- `day_start_mode` accepts `fixed` or `sunrise`.
- `first_day_of_week` accepts `monday` through `sunday`.

## Quality Gates

Before opening a PR, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```
