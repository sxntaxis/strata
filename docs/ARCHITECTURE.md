# Strata architecture authority

Status: current implementation map
Last reviewed: 2026-08-02

## Current system

```text
TUI / CLI
    ↓
shared invocation and validated startup configuration
    ↓
application orchestration
    ↓
domain time, category, session, report, recovery, and snapshot rules
    ↓
SQLite repository/runtime coordination + sediment simulation
```

Current responsibility map:

- `src/main.rs` — process entry.
- `src/lib.rs` — shared CLI/TUI invocation and startup authority.
- `src/cli.rs` — command lifecycle, reports, exports, migration, and maintenance.
- `src/keybindings.rs` — keymap plus validated runtime/time settings.
- `src/domain.rs` — canonical sessions, project/category identity, operational-day allocation, and reports.
- `src/temporal.rs` — monotonic/wall reconciliation, fixed-offset civil policy, and exact overlap slicing.
- `src/sqlite.rs` and `src/sqlite/**` — schema migrations, repositories, runtime transactions, checkpoint custody, deterministic interchange, backup/restore, and fault certification.
- `src/storage.rs` — XDG paths, legacy-file authority, atomic file helpers, and custody-separated contribution files.
- `src/app.rs` and `src/app/**` — TUI orchestration, persistence reconciliation, bounded recovery, historical artifact selection, interaction, and rendering.
- `src/sand/engine.rs` — canonical logical grains, compressed pending mass, physics, viewport projection, and Braille rendering.
- `src/sand/recovery.rs` — bounded recovery arithmetic and topology-preserving detached contribution.
- `src/sand/snapshot.rs` — snapshot kinds, exact daily contribution construction, provenance, revisions, selection, and immutable rendering.

## Established authority

### Persistence and startup

- SQLite becomes live authority only after explicit activation.
- CLI and TUI share one validated startup configuration.
- Activated runtime never dual-writes or silently falls back to legacy sources.
- Persistence and authority failures fail closed with visible recovery controls.
- Deterministic CSV bundles are interchange, not a competing live ledger.

### Time and sessions

- Live duration is monotonic; UTC owns persisted absolute chronology.
- Fixed-offset civil policy owns new operational-day projection.
- Canonical sessions remain singular while exact overlap slices allocate report and daily-contribution mass across operational days.
- Project and category are independent canonical axes.
- Idle is explicit, continues producing sediment, and remains excluded from ordinary active-time totals.

### Reports and exports

- Report ranges are inclusive operational-day projections.
- Active time is included by default as explicit provisional state; `--completed-only` selects committed history.
- Ordering is deterministic.
- JSON schema version 2 and RFC 5545-safe ICS use stable identities and authoritative UTC endpoints.

### Sediment mass and topology

- Every due grain is exactly one placed or pending logical grain.
- Pending mass uses ordered category/count runs.
- Terminal-cell and Braille-dot dimensions are distinct.
- The persisted logical grid owns canonical topology.
- Resize is a centered, bottom-aligned projection-only operation.
- Blockage, resize, persistence, restore, and recovery conserve total and per-category mass.

### Runtime recovery

- Runtime checkpoints cover autosave, detach, terminal closure, and crash recovery.
- Evidence is claimed and a fixed target is persisted before recovered publication.
- Checkpoint topology and engine metadata restore directly.
- Missed mass becomes compressed pending runs.
- Missed physics is never replayed.
- SQLite recovery publication is atomic and reclaimable.
- Legacy-file retry is deterministic.
- Recovery completion reconciles every operational day touched by canonical slices.

### Historical sediment artifacts

`SedimentSnapshot` distinguishes:

- cumulative checkpoints;
- daily contributions;
- derived previews.

Each artifact records day, revision, provenance, idle policy, reconstruction status, and `SandState`. Kinds are non-interchangeable. Historical viewing is immutable and never advances physics or writes persistence.

### Daily contribution authority

Persisted daily sediment is derived from exact canonical session slices rather than cumulative live state.

- Idle inclusion is explicit.
- Every second is conserved, with overflow represented as pending runs.
- Source revision covers all sediment-relevant chronology and identity fields.
- Persisted artifacts are trusted only on exact schema/kind/day/revision match.
- Missing or stale authority yields an in-memory derived preview until reconciliation publishes the correct contribution.
- Autosave, full-state flush, relevant deletion, and recovery completion reconcile authoritative contributions.
- Cross-boundary session deletion rebuilds every touched day.
- Description-only edits do not invalidate sediment.

SQLite schema version 6 introduces `snapshot_kind = 'daily-contribution'` while preserving old `daily` rows. Legacy-file authority uses `.contribution.json` while preserving old daily JSON files. Legacy cumulative artifacts are archive-in-place evidence and never silently become new authority.

The detailed contract is `docs/SEDIMENT_AUTHORITY.md`.

## Truth boundaries

### Chronological ledger

Owns exact elapsed intervals, timestamps, categories, projects, descriptions, operational-day policy, and reportable totals.

### Sediment formation

Owns accountable visual history and canonical topology. It must conserve mass and category identity while remaining independent of the current viewport.

### Runtime recovery

Owns checkpoint evidence and exact elapsed contribution since the checkpoint. It may add mass and advance accumulator remainders, but may not replay unbounded physics, relax topology, or discard unresolved evidence.

### Historical snapshots

Own semantic identity and provenance for persisted or derived visual artifacts. A derived preview is a read-only projection; a daily contribution becomes authority only through explicit typed persistence.

### Interface

TUI and CLI translate user intent and present state. Neither may own an independent ledger, reinterpret authority, mutate canonical sediment to fit the terminal, or advance historical artifacts while viewing them.

## Current architectural frontier

The sediment conservation program is complete. The next priorities are:

1. INTERACTION-001 — explicit edit modes, truthful keybinding behavior, and terminal lifecycle safety;
2. reconciliation of partially satisfied issues #5, #10, and #13;
3. later domain/profile work, including complete profile isolation under issue #15.

## Non-authority

- GitHub issues do not override accepted doctrine.
- Notebook research is working memory until promoted.
- Terminal dimensions are not canonical sediment dimensions.
- A derived preview is not persisted authority.
- Legacy cumulative daily rows/files are evidence, not daily contributions.
- CSV, JSON, and ICS are external adapters, not canonical domain models.
