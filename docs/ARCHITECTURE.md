# Strata architecture authority

Status: current implementation map
Last reviewed: 2026-08-01

## Current system

Strata is one Rust application with two user interfaces:

```text
TUI / CLI
    ↓
application orchestration and validated settings
    ↓
domain time, layer, session, and report rules
    ↓
SQLite repository/runtime coordination + sediment simulation
```

Current responsibility map:

- `src/main.rs` — process entry and TUI/CLI selection.
- `src/cli.rs` — command parsing, non-interactive lifecycle, reports, exports, migration, and maintenance commands.
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

The closure evidence is `docs/SQLITE_MIGRATION_CLOSURE_AUDIT.md`.

## Truth boundaries

### Chronological ledger

Owns exact elapsed intervals, timestamps, layers, project identity, notes, operational-day interpretation, and reportable totals.

### Sediment formation

Owns accountable visual history. Total represented duration and per-layer mass must be conserved exactly. Topology, contours, color composition, neighborhoods, and broad chronology require explicit preservation contracts.

### Reports and balance

Are projections over chronological truth. They may omit idle or apply user-defined polarity, but they must not rewrite underlying intervals.

### Interface

TUI and CLI translate user intent and present state. Neither may maintain an independent ledger or silently select a different profile.

## Current architectural frontier

Persistence structure is no longer the primary risk. The next program is authority and temporal correctness:

1. validate configuration/profile selection before any database is opened;
2. establish one explicit time authority and clock-jump policy;
3. correct interval allocation, reporting, export, and classification semantics;
4. establish a conserved sediment model independent of viewport and mutable previews.

## Non-authority

- GitHub issues describe defects and proposals; they do not override accepted product doctrine.
- Notebook research remains working memory until promoted.
- Sediment snapshots and report output are projections, not substitutes for their owning state.
- CSV, JSON, and ICS are external adapters, not canonical domain models.
