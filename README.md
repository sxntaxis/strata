# Strata

Strata is a Rust continuous temporal ledger and active timer expressed through a falling-sand terminal UI, with a small CLI for non-interactive use.

Time remains present as **idle** sediment when no active layer is selected. Selecting a layer gives the continuing flow an activity identity and makes Strata function as a timer. The exact ledger and the accountable sediment formation are both historically meaningful at different levels of precision.

Accepted product and architecture direction lives in [`docs/`](docs/PROJECT.md). Developing thought, evidence, decisions, and current work live in the repository-local [`notebook/`](notebook/README.md).

## Goals

- Keep the behavior stable and predictable.
- Keep the codebase easy to read and review.
- Keep domain, storage, and UI concerns separated.
- Preserve exact chronological history and accountable sedimentary history.
- Treat the artistic system as product function rather than decoration.

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
- `src/storage.rs`: current persistence (CSV/JSON, paths, atomic writes, backups).
- `src/app.rs` + `src/app/*`: TUI orchestration, rendering, and key handling.
- `src/cli.rs`: command handling and output formatting for non-TUI usage.
- `src/sand/*`: sand simulation and rendering primitives.

When changing code, keep these boundaries strict:

- Domain does not perform file I/O.
- Storage does not contain UI behavior.
- UI orchestrates, but avoids embedding core business logic.

SQLite is the accepted future live authority; deterministic CSV remains a first-class import/export format. See GitHub issue #8 and [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md). The migration is not yet implemented.

## Data Locations

Strata stores runtime data in XDG paths:

- Data: `~/.local/share/strata/`
- State: `~/.local/state/strata/`
- Config: `~/.config/strata/`

You can override the data directory with `STRATA_DATA_DIR=/your/path`.

Repo-local runtime artifacts are intentionally ignored by git.

## Keybindings

- Open the Command Atlas in TUI with `?` or `F1`.
- Open the Command Palette with `Ctrl+P`.
- Open Karma with `k`.
- In main view, `d` detaches Strata while tracking continues.
- In Karma pop-up, `d` or `t` sets day range, `w` week, and `m` month.
- In layer text entry, `?` stays a normal character; use `F1` there.
- Optional config file: `~/.config/strata/keymap.json`
- In Karma pop-up, `←` moves to older intervals and `→` moves toward current.

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
- `toggle_command_palette` is the action name for rebinding palette open/close.

Karma interval notes:

- `month` uses calendar months (current month-to-date, then full prior months).
- Daily sand snapshots are written under state `sand_history/`; if active layer is drift, drift grains are filtered from the saved daily snapshot to reduce sleep-noise history.
- If a historical day snapshot is missing, Strata reconstructs an approximate snapshot from that day in `time_log.csv`.

Detached mode notes:

- Detach writes a runtime checkpoint under state and exits TUI without ending the active session.
- Reopening Strata resumes visualization and catches up in background at fixed cadence (24x) with a centered bottom line gauge.
- During catch-up, mutating main-view actions are queued and replayed when simulation time reaches them.
- Catch-up view renders settled sediment projection (quiet/no falling animation), then switches to live falling dots once synced.

The public rename from drift to idle, the Karma terminology, sunrise behavior, and detached-mode contract are active product or implementation work. Current runtime behavior above is documented rather than endorsed as final doctrine.

## Quality Gates

Before opening a PR, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```
