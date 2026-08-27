# Strata

Strata is a Rust time tracker with a terminal UI and a small CLI.

## Goals

- Keep behavior stable and predictable.
- Preserve classified history and session identity through explicit, transactional persistence.
- Keep domain, repository, and UI concerns separated.

## Build and run

```bash
cargo run
```

Run CLI commands with arguments:

```bash
cargo run -- report --today
```

### CLI tracking

Strata classifies elapsed time by **layer/category**. A layer owns the activity identity, color, and balance direction. Start or switch directly by layer name or ID:

```bash
strata start Work --desc "Implementation"
strata stop
strata report --today
```

`stop` returns the continuous ledger to **idle** rather than creating unclassified time. Idle is category ID `0`: it remains part of sediment history while being excluded from ordinary active-time totals.

## Architecture

- `src/domain.rs`: business rules for categories, sessions, operational days, and reports.
- `src/sqlite.rs` and `src/sqlite/*`: the current SQLite schema, repository operations, maintenance, runtime coordination, and TUI adapters.
- `src/storage.rs`: profile paths and atomic helpers for current configuration/state publication.
- `src/app.rs` and `src/app/*`: TUI orchestration, rendering, event handling, and persistence-recovery controls.
- `src/cli.rs`: command parsing and non-TUI output.
- `src/sand/*`: sediment simulation and rendering primitives.

Boundary rules:

- Domain code does not perform file or database I/O.
- SQLite owns all authoritative transactional runtime persistence.
- UI and CLI call repository/runtime adapters rather than issuing ad hoc SQL.
- Portable CSV bundles are interchange, never runtime authority.

## Data locations

Strata uses XDG paths:

- Data: `~/.local/share/strata/`
- State: `~/.local/state/strata/`
- Config: `~/.config/strata/`

Select a complete profile with `--profile /path/to/profile` or `STRATA_PROFILE=/path/to/profile`. Profile selection owns data, state, and configuration together; partial data-path redirection is rejected.

The normal SQLite database is stored at `data/strata.sqlite3` under the selected profile. Recovery exports and runtime state use the corresponding profile state root. Repo-local runtime artifacts are intentionally ignored by Git.

## Configuration authority

Strata loads and validates `~/.config/strata/keymap.json` once before choosing the CLI or TUI and before opening the profile database. Malformed JSON, unknown keys or actions, invalid operational-day settings, and unsupported UTC offsets stop startup with a non-zero error that identifies the file and invalid value.

Strata does not silently replace a broken configuration with defaults. To deliberately ignore the file for one invocation, use the global override:

```bash
strata --ignore-config report --today
strata --ignore-config start Work
strata --ignore-config
```

The override uses built-in settings intentionally; normal XDG or complete-profile selection still applies. During a running TUI session, a failed configuration reload keeps the last valid settings and displays the error instead of applying a partial configuration.


## Time authority

Strata uses distinct clocks for distinct truths:

- **Live elapsed duration** uses the process monotonic clock.
- **Persisted timestamps** use UTC.
- **Civil start/end rendering and operational-day allocation** use the validated fixed UTC offset from `keymap.json`.
- **Historical allocation** uses each completed session's persisted fixed-offset boundary policy and absolute interval; later setting changes do not redivide old history.

At a live finish or layer switch, Strata reconciles monotonic elapsed time against observed UTC wall time. A divergence greater than five seconds is treated as a clock discontinuity: the transition fails visibly and active state remains available for recovery rather than being converted into ordinary work.

CLI stops and recovered sessions cannot reconstruct a cross-process monotonic clock, so they use a checked UTC wall interval. Future starts are rejected. An unattended interval above seven days requires explicit confirmation:

```bash
strata stop --accept-clock-jump
```

Use that override only after inspecting the active timestamp and system clock; it accepts the recorded wall interval rather than guessing a correction.

The current policy is a **fixed clock under a fixed UTC offset**, not an IANA timezone. It is deterministic across travel and seasonal clock changes but does not automatically apply daylight-saving transitions. The former `sunrise` option never performed solar calculation and has been removed; an existing `day_start_mode: "sunrise"` setting is rewritten visibly to `fixed` while preserving its configured hour and minute.

A completed session remains one canonical ledger row. Reports project exact overlap slices at operational-day boundaries using the policy stored with that session, so a cross-boundary interval contributes only its overlapping seconds to each day without losing identity or creating empty exact-boundary fragments. Transitions whose whole-second duration is zero still complete or switch active state transactionally, but they do not create ordinary work rows. The full contract is recorded in [`docs/TEMPORAL_AUTHORITY.md`](docs/TEMPORAL_AUTHORITY.md).

## Reporting and exports

Reports are projections over canonical ledger truth. Preset reports use the current operational day, configured week-to-date, or calendar month-to-date. Custom ranges are inclusive operational-day ranges:

```bash
strata report --from 2026-07-01 --to 2026-07-15
```

A running interval is included by default as provisional time and is identified explicitly in report output and exports. Use committed history only when required:

```bash
strata report --today --completed-only
strata export --format json --completed-only
```

JSON export schema version 4 includes stable event UIDs, authoritative UTC endpoints, category identity, and a `provisional` flag. ICS export uses those UTC endpoints and stable UIDs, emits CRLF-delimited RFC 5545 text with escaping and line folding, marks provisional events with `X-STRATA-PROVISIONAL:TRUE`, and excludes idle events. A session without authoritative absolute chronology fails closed for ICS rather than inventing timestamps.

Week reports follow the configured first day of week. The current week is week-to-date; prior week offsets in the TUI are complete calendar weeks. Month reports use calendar months: the current month is month-to-date and prior offsets are complete prior calendar months.

The detailed contract is recorded in [`docs/REPORT_AUTHORITY.md`](docs/REPORT_AUTHORITY.md).

## Persistence authority

Every selected profile opens or creates `data/strata.sqlite3` directly. The database is bound to the
profile UUID and a mismatched database fails closed. CLI and TUI share this database; there is no
activation ceremony, migration step, fallback, or second runtime authority. A non-current development
schema is rejected rather than upgraded.

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

## Persistence failure recovery

When an authoritative TUI write fails, Strata freezes normal mutation and displays a non-dismissible recovery surface. Available actions include retry, authoritative reload, emergency custody export, safe export-and-exit, and explicit exit without saving.

The emergency JSON bundle is a custody artifact generated from current application state. It is not the same as the portable CSV bundle and is not a supported import format.

## Keybindings

- Open the Command Atlas in TUI with `?` or `F1`.
- Open the Command Palette with `Ctrl+P`.
- Open Balance with `b`.
- In main view, `d` detaches Strata while tracking continues.
- In Balance, `d` or `t` selects day range, `w` week, and `m` month.
- In layer text entry, `?` remains a normal character; use `F1` there.
- Optional config file: `~/.config/strata/keymap.json`.
- In Balance, `←` moves to older intervals and `→` moves toward current.

Example:

```json
{
  "keymap_inherit": true,
  "day_start_mode": "fixed",
  "day_start_hour": 6,
  "day_start_minute": 0,
  "first_day_of_week": "monday",
  "unbind_actions": ["open_layer_popup"],
  "keymap": {
    "f": "open_balance_popup",
    "b": null,
    "ctrl-q": "quit"
  }
}
```

Notes:

- `keymap_inherit: true` starts from built-in defaults, then applies overrides.
- Setting a key to `null` unbinds that key.
- `unbind_actions` disables specific actions by name.
- Setting `keymap_inherit: false` starts from an empty keymap.
- `day_start_mode` accepts only `fixed`. Existing `sunrise` values are migrated visibly to `fixed`; Strata never implemented solar sunrise calculation.
- `first_day_of_week` accepts `monday` through `sunday`.
- `toggle_command_palette` is the action name for rebinding palette open/close.

Balance interval notes:

- `month` uses calendar months: current month-to-date, then complete prior calendar months.
- Day, week, and month totals allocate canonical sessions by exact overlap with their persisted operational-day boundary policy.
- A zero-whole-second finish or switch is a transition event, not a completed work row.
- Daily sediment snapshots are authoritative SQLite records.
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
