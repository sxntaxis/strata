# Strata

Strata is a Rust time tracker with a terminal UI and a small CLI.

## Goals

- Keep behavior stable and predictable.
- Preserve billable history through explicit, transactional persistence.
- Keep domain, repository, and UI concerns separated.

## Build and run

```bash
cargo run
```

Run CLI commands with arguments:

```bash
cargo run -- report --today
```

## Architecture

- `src/domain.rs`: business rules for categories, sessions, operational days, and reports.
- `src/sqlite.rs` and `src/sqlite/*`: SQLite schema, repository operations, migration, authority activation, maintenance, runtime coordination, and TUI adapters.
- `src/storage.rs`: XDG paths, legacy CSV/JSON compatibility needed before migration, and atomic file helpers.
- `src/app.rs` and `src/app/*`: TUI orchestration, rendering, event handling, and persistence-recovery controls.
- `src/cli.rs`: command parsing and non-TUI output.
- `src/sand/*`: sediment simulation and rendering primitives.

Boundary rules:

- Domain code does not perform file or database I/O.
- SQLite owns authoritative transactional persistence after activation.
- UI and CLI call repository/runtime adapters rather than issuing ad hoc SQL.
- Legacy CSV/JSON is never dual-written after SQLite activation.

## Data locations

Strata uses XDG paths:

- Data: `~/.local/share/strata/`
- State: `~/.local/state/strata/`
- Config: `~/.config/strata/`

You can override the data directory with `STRATA_DATA_DIR=/your/path`.

The normal SQLite candidate is stored under the Strata data directory. Migration reports, authority metadata, immutable source backups, recovery exports, and runtime state use the corresponding data/state roots. Repo-local runtime artifacts are intentionally ignored by Git.

## Persistence authority

Strata has two explicit authority phases:

1. **Legacy authority** — existing CSV/JSON files remain live until migration and activation are completed.
2. **SQLite authority** — after explicit activation, both CLI and TUI use one SQLite database. Legacy files remain unchanged as migration evidence and are not dual-written.

Strata does not automatically migrate during startup, replace a damaged database with an empty one, or fall back from SQLite to stale legacy files.

### Migration sequence

Validate the legacy state without writing artifacts:

```bash
strata migrate-sqlite --dry-run
```

Publish a verified SQLite candidate, immutable source backup, migration report, and authority marker:

```bash
strata migrate-sqlite
```

Activate the verified candidate explicitly:

```bash
strata activate-sqlite --confirm
```

Active or detached recovery state requires explicit migration opt-in:

```bash
strata migrate-sqlite --include-active-recovery
```

Use `--json` on migration and activation commands for machine-readable reports.

## Portable CSV interchange

SQLite data can be exported as a deterministic, versioned directory bundle:

```bash
strata sqlite-export --out ./strata-bundle
```

The bundle contains a manifest plus deterministic CSV files for categories, category tags, sessions, active state, runtime checkpoints, current sediment state, and sediment snapshots.

Import a validated bundle into a **new** SQLite database:

```bash
strata sqlite-import --bundle ./strata-bundle --database ./restored.sqlite3
```

Import validates manifest fingerprints, file sizes, schemas, identities, references, totals, and repository-snapshot parity. Existing targets are not overwritten.

Current limitation: portable import does not yet expose a validation-only `--dry-run` mode. This is tracked as a migration-program closure requirement in [`docs/SQLITE_MIGRATION_CLOSURE_AUDIT.md`](docs/SQLITE_MIGRATION_CLOSURE_AUDIT.md).

The general `strata export --format ...` command remains for JSON and ICS session exports; full-fidelity CSV interchange uses `sqlite-export` and `sqlite-import`.

## Database maintenance

Check schema, integrity, foreign keys, and authority metadata:

```bash
strata sqlite-doctor
```

Create a verified standalone backup:

```bash
strata sqlite-backup --out ./strata-backup.sqlite3
```

Verify and atomically restore a backup:

```bash
strata sqlite-restore --backup ./strata-backup.sqlite3 --replace
```

Maintenance operations use explicit locking and refuse stale temporary artifacts or active SQLite sidecars where publication would be unsafe. Add `--json` for machine-readable reports.

## Persistence failure recovery

When an authoritative TUI write fails, Strata freezes normal mutation and displays a non-dismissible recovery surface. Available actions include retry, authoritative reload, emergency custody export, safe export-and-exit, and explicit exit without saving.

The emergency JSON bundle is a custody artifact generated from current application state. It is not the same as the portable CSV bundle and is not yet a supported import format.

## Legacy evidence retention

Migration and activation preserve legacy source bytes. After activation, those files are evidence rather than live authority.

Current limitation: Strata does not yet provide a supported command to inventory, archive, or remove verified legacy evidence. Do not assume that deleting every CSV/JSON file in the data directory is safe; custom paths and unrelated files may exist. The bounded closure work is defined in the SQLite migration audit linked above.

## Keybindings

- Open the Command Atlas in TUI with `?` or `F1`.
- Open the Command Palette with `Ctrl+P`.
- Open Karma with `k`.
- In main view, `d` detaches Strata while tracking continues.
- In Karma, `d` or `t` selects day range, `w` week, and `m` month.
- In layer text entry, `?` remains a normal character; use `F1` there.
- Optional config file: `~/.config/strata/keymap.json`.
- In Karma, `←` moves to older intervals and `→` moves toward current.

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
- `time_log_path` configures the legacy CSV source before SQLite activation and is migration input afterward.
- `day_start_mode` accepts `fixed` or `sunrise`.
- `first_day_of_week` accepts `monday` through `sunday`.
- `toggle_command_palette` is the action name for rebinding palette open/close.

Karma interval notes:

- `month` uses calendar months: current month-to-date, then complete prior calendar months.
- Daily sediment snapshots are authoritative SQLite records after activation.
- If a historical snapshot is missing, Strata reconstructs an approximation from that day's completed sessions.

Detached mode notes:

- Detach preserves the active session and writes a runtime checkpoint.
- Under SQLite authority, checkpoint claim, catch-up sediment publication, daily snapshot publication, and checkpoint completion are transactionally coordinated.
- A failed recovery commit leaves the checkpoint reclaimable on the next launch.
- During catch-up, mutating main-view actions are queued and replayed when simulation time reaches them.

## Quality gates

Before opening a PR, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```
