# Strata architecture authority

Status: current implementation map
Last reviewed: 2026-08-01

## Current system

Strata is one Rust application with two user interfaces:

```text
TUI / CLI
    ↓
shared invocation and validated startup configuration
    ↓
application orchestration
    ↓
domain time, layer, session, and report rules
    ↓
SQLite repository/runtime coordination + sediment simulation
```

Current responsibility map:

- `src/main.rs` — process entry.
- `src/lib.rs` — shared CLI/TUI invocation, startup configuration validation, and entry-point selection.
- `src/cli.rs` — command parsing, non-interactive lifecycle, reports, exports, migration, and maintenance commands.
- `src/keybindings.rs` — shared keymap and authority/time-setting parsing and validation.
- `src/domain.rs` — categories, sessions, operational-day logic, and report aggregation.
- `src/sqlite.rs` and `src/sqlite/**` — schema migration, authoritative repositories, CLI/TUI adapters, runtime coordination, failure certification, deterministic interchange, backup/restore, and legacy-evidence custody.
- `src/storage.rs` — XDG paths, pre-activation legacy compatibility, migration input, and atomic file helpers.
- `src/app.rs` and `src/app/**` — TUI orchestration, interaction, rendering, reports, modals, and persistence-recovery controls.
- `src/sand/**` — logical grains, physics, resizing, snapshots, and Braille rendering.

## Authority state

The SQLite migration program closed at 9/9 acceptance criteria in issue #8.

- Before explicit activation, legacy CSV/JSON remains the live authority.
- After explicit activation, CLI and TUI share one SQLite authority.
- Activated runtime never dual-writes legacy sources and never falls back to them automatically.
- Runtime transitions are stable-ID fenced, transactional, and retry-safe.
- Persistence failures enter a non-dismissible recovery state rather than continuing with divergent memory.
- Deterministic CSV bundles are interchange, not a competing live ledger.
- Legacy source inventory, archive, and removal are provenance-verified custody operations.

AUTHORITY-001 adds a shared startup gate before either interface resolves or opens data authority:

- one top-level invocation chooses CLI or TUI;
- `keymap.json` is loaded and validated once;
- malformed JSON, unknown key/action data, invalid day/time settings, unsupported UTC offsets, and invalid configured legacy paths stop startup visibly;
- CLI and TUI receive the same validated runtime/storage settings;
- `--ignore-config` is the only deliberate built-in-default bypass;
- TUI hot-reload failures retain the last valid settings rather than applying a partial configuration.

The SQLite closure evidence is `docs/SQLITE_MIGRATION_CLOSURE_AUDIT.md`.

## Truth boundaries

### Chronological ledger

Owns exact elapsed intervals, timestamps, layers, project identity, notes, operational-day interpretation, and reportable totals.

### Sediment formation

Owns accountable visual history. Total represented duration and per-layer mass must be conserved exactly. Topology, contours, color composition, neighborhoods, and broad chronology require explicit preservation contracts.

### Reports and balance

Are projections over chronological truth. They may omit idle or apply user-defined polarity, but they must not rewrite underlying intervals.

### Interface

TUI and CLI translate user intent and present state. Neither may maintain an independent ledger, independently reinterpret configuration, or silently select a different profile.

## Current architectural frontier

Persistence structure and startup configuration fallback are no longer the primary risks. The next program is temporal correctness:

1. establish one explicit time authority and wall-clock-jump policy;
2. define timezone and historical operational-day reproducibility;
3. correct interval allocation, reporting, export, and classification semantics;
4. establish a conserved sediment model independent of viewport and mutable previews.

Complete profile isolation and deliberate runtime profile switching remain separate work under issue #15.

## Non-authority

- GitHub issues describe defects and proposals; they do not override accepted product doctrine.
- Notebook research remains working memory until promoted.
- Sediment snapshots and report output are projections, not substitutes for their owning state.
- CSV, JSON, and ICS are external adapters, not canonical domain models.
