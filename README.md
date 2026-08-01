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
- `src/sqlite.rs` and `src/sqlite/*`: SQLite schema, repository operations, migration, authority activation, maintenance, runtime coordination, legacy-evidence custody, and TUI adapters.
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

The normal SQLite database is stored under the Strata data directory. Migration reports, authority metadata, immutable source backups, removal ledgers, recovery exports, and runtime state use the corresponding data/state roots. Repo-local runtime artifacts are intentionally ignored by Git.

## Configuration authority

Strata loads and validates `~/.config/strata/keymap.json` once before choosing the CLI or TUI and before resolving a writable data authority. Malformed JSON, unknown keys or actions, invalid operational-day settings, unsupported UTC offsets, and invalid configured legacy paths stop startup with a non-zero error that identifies the file and invalid value.

Strata does not silently replace a broken configuration with defaults. To deliberately ignore the file for one invocation, use the global override:

```bash
strata --ignore-config report --today
strata --ignore-config start project-a --category Work
strata --ignore-config
```

The override uses built-in settings intentionally; normal XDG and `STRATA_DATA_DIR` environment selection still applies. During a running TUI session, a failed configuration reload keeps the last valid settings and displays the error instead of applying a partial configuration.


## Time authority

Strata uses distinct clocks for distinct truths:

- **Live elapsed duration** uses the process monotonic clock.
- **Persisted timestamps** use UTC.
- **Civil start/end rendering and operational-day allocation** use the validated fixed UTC offset from `keymap.json`.
- **Historical report grouping** uses the operational-day key persisted with each completed session; later offset changes do not regroup old history.

At a live finish or layer switch, Strata reconciles monotonic elapsed time against observed UTC wall time. A divergence greater than five seconds is treated as a clock discontinuity: the transition fails visibly and active state remains available for recovery rather than being converted into ordinary work.

CLI stops and recovered sessions cannot reconstruct a cross-process monotonic clock, so they use a checked UTC wall interval. Future starts are rejected. An unattended interval above seven days requires explicit confirmation:

```bash
strata stop --accept-clock-jump
```

Use that override only after inspecting the active timestamp and system clock; it accepts the recorded wall interval rather than guessing a correction.

The current policy is a **fixed offset**, not an IANA timezone. It is deterministic across travel and seasonal clock changes but does not automatically apply daylight-saving transitions. Sunrise semantics remain separate work. The full contract is recorded in [`docs/TEMPORAL_AUTHORITY.md`](docs/TEMPORAL_AUTHORITY.md).

## Persistence authority

Strata has two explicit authority phases:

1. **Legacy authority** — existing CSV/JSON files remain live until migration and activation are completed.
2. **SQLite authority** — after explicit activation, both CLI and TUI use one SQLite database. Legacy files become preserved migration evidence and are not dual-written.

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

Validate the full import without publishing a target database:

```bash
strata sqlite-import --bundle ./strata-bundle --dry-run
```

Dry-run uses the same parser, temporary SQLite import, integrity check, and repository-snapshot reconciliation as a real import. It does not create the requested database, its parent directory, a publication lock, or an authority marker.

Import the validated bundle into a **new** SQLite database:

```bash
strata sqlite-import --bundle ./strata-bundle --database ./restored.sqlite3
```

Import validates manifest fingerprints, file sizes, schemas, identities, references, totals, and repository-snapshot parity. Existing targets are not overwritten. Use `--json` for a machine-readable validation or import report.

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

## Legacy migration evidence

Migration preserves an immutable fingerprinted backup and leaves the original CSV/JSON files untouched. After activation, those originals are evidence rather than live authority.

Inspect the verified evidence set:

```bash
strata sqlite-legacy-inventory
```

Inventory requires an active, healthy SQLite authority. It reconciles:

- the authority marker and activation provenance;
- SQLite migration metadata and the verified source manifest stored in the database;
- the immutable migration backup and `source_paths.json`;
- every live source path, byte count, and content fingerprint.

Changed, missing, symlinked, redirected, or unprovenanced sources fail closed.

Archive verified evidence before considering removal:

```bash
strata sqlite-legacy-archive --out ./strata-legacy-evidence --confirm
```

The archive is built from the immutable migration backup, not from potentially changed live files. Publication uses a fingerprint-owned staging directory, verifies every archived byte, and is idempotent. A complete interrupted publication is finished on retry; an owned partial stage is safely rebuilt. Foreign staging directories are never removed automatically.

Original source removal is a separate irreversible command:

```bash
strata sqlite-legacy-remove \
  --archive ./strata-legacy-evidence \
  --confirm-fingerprint <MIGRATION_FINGERPRINT>
```

Removal requires all of the following:

- active SQLite authority matching the original verified migration;
- a fully verified archive;
- the exact migration fingerprint printed by inventory/archive;
- unchanged original source bytes.

A durable removal ledger makes interrupted multi-file deletion retryable. Strata removes only the exact paths recorded by the verified SQLite source manifest; unrelated files in the same directories are never selected. The immutable migration backup, archive, SQLite database, authority marker, and removal ledger remain available afterward.

All three evidence commands support `--authority-marker <PATH>` and `--json`.

## Persistence failure recovery

When an authoritative TUI write fails, Strata freezes normal mutation and displays a non-dismissible recovery surface. Available actions include retry, authoritative reload, emergency custody export, safe export-and-exit, and explicit exit without saving.

The emergency JSON bundle is a custody artifact generated from current application state. It is not the same as the portable CSV bundle and is not a supported import format.

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
- `time_log_path` configures the legacy CSV source before SQLite activation and is migration provenance afterward.
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

## SQLite migration closure

The full acceptance reconciliation is recorded in [`docs/SQLITE_MIGRATION_CLOSURE_AUDIT.md`](docs/SQLITE_MIGRATION_CLOSURE_AUDIT.md). SQLITE-012 completes all nine acceptance criteria from issue #8 without automatic migration, legacy dual writes, or emergency-JSON import scope.

## Quality gates

Before opening a PR, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```
