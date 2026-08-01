# Strata architecture authority

Status: current implementation map with accepted migration directions
Last reviewed: 2026-08-01

## Current system

Strata is one Rust application with two user interfaces:

```text
TUI / CLI
    ↓
application orchestration
    ↓
domain time, layer, session, and report rules
    ↓
storage adapters and sediment simulation
```

Current responsibility map:

- `src/main.rs` — process entry and TUI/CLI selection.
- `src/cli.rs` — command parsing, non-interactive lifecycle, reports, and exports.
- `src/domain.rs` — layers/categories, sessions, operational-day logic, and report aggregation.
- `src/storage.rs` — current CSV/JSON paths, reads, writes, backups, and runtime state files.
- `src/app.rs` and `src/app/**` — TUI orchestration, interaction, rendering, reports, modals, and runtime transitions.
- `src/sand/**` — logical grains, physics, resizing, snapshots, and Braille rendering.

The source and verified runtime remain implementation reality. This document does not claim that the current boundaries are already ideal.

## Accepted persistence direction

The current CSV/JSON collection is authoritative only until the migration tracked in GitHub issue #8 is implemented and verified.

The accepted target direction is:

- one versioned SQLite database as the live source of truth;
- one repository API shared by CLI and TUI;
- transactional active-session, stop, switch, detach, recovery, and edit transitions;
- stable identities and integrity constraints;
- first-class deterministic CSV import and export;
- validated one-time legacy import with preserved backups;
- visible failures rather than writable empty fallback state.

The migration must not treat SQLite as permission to preserve incorrect domain semantics. Product and interaction issues remain separate work.

## Truth boundaries

### Chronological ledger

Owns exact elapsed intervals, timestamps, layers, notes, operational-day interpretation, and reportable totals.

### Sediment formation

Owns accountable visual history. Total represented duration and per-layer mass must be conserved exactly. Topology, contours, color composition, neighborhoods, and broad chronology must survive administrative operations according to explicit tolerances.

### Reports and balance

Are projections over chronological truth. They may omit idle or apply user-defined polarity, but they must not rewrite the underlying elapsed intervals.

### Interface

TUI and CLI translate user intent and present state. Neither may maintain an independent competing ledger.

## Current migration pressure

The audit recorded reliability, lifecycle, export, simulation, configuration, timekeeping, and interaction defects in GitHub issues #2 through #28. The active implementation sequence is governed from `notebook/work/RELIABILITY-001-persistence-and-audit-remediation.md`.

## Non-authority

- GitHub issues describe defects and proposals; they do not override accepted product doctrine.
- Notebook research remains working memory until promoted.
- Sediment snapshots and report output are projections, not substitutes for their owning state.
- External formats such as CSV and ICS are adapters, not canonical domain models.
