# Sediment authority

Status: implemented and certified
Program: SEDIMENT-001
Current completed unit: SEDIMENT-001D2
Issues completed: #6, #7, #16, #18, #26
Last reviewed: 2026-08-02

## Purpose

Sediment is accountable visual history derived from elapsed time. Chronological sessions remain the exact time authority; sediment preserves explicit mass, category, topology, recovery, snapshot, and projection obligations.

## Logical mass

Every due grain exists exactly once as either:

- a placed grain in the canonical logical dot grid; or
- a pending grain waiting for ordinary live ingress.

Physical blockage never authorizes loss. `grain_count` represents placed plus pending mass. Pending grains retain category identity and FIFO category order.

Pending mass is stored as ordered category/count runs. Adjacent additions for the same category merge, while category transitions remain ordered. Bulk addition and persistence are independent of represented count, apart from bounded placement into currently free ingress columns. Count overflow fails visibly.

Clearing all sediment clears placed and pending mass. Category clearing and counted removal apply to both forms.

## Geometry and canonical topology

Terminal-cell dimensions and Braille-dot dimensions are distinct units:

- `cell_width` and `cell_height` are viewport dimensions;
- `grid_width_dots` and `grid_height_dots` are canonical logical-canvas dimensions;
- one terminal cell projects `dot_width × dot_height` logical dots.

The persisted logical grid owns coordinates, neighborhoods, category composition, and topology. Terminal resize changes viewport state only. Shrink crops presentation without deleting hidden grains; expansion pads without stretching or relocating grains. Projection is horizontally centered and bottom-aligned.

Resize never invokes gravity, ingress placement, repacking, overflow insertion, or another canonical mutation. Repeated resize oscillation with no elapsed time is exactly idempotent at the `SandState` level.

## SandState persistence

`SandState` schema version 2 stores ordered pending runs.

- Version 1 `pending_grains` vectors migrate deterministically into adjacent runs.
- Older JSON with no pending field loads as an empty reservoir.
- Empty pending collections are omitted during serialization.
- Canonical grid dimensions restore as persisted, independently of the opening terminal.
- Ordinary restore normalizes unavailable category identities to idle; checkpoint recovery is stricter and refuses unavailable identities.

## Bounded runtime recovery

Runtime checkpoints cover periodic autosave, detach, terminal closure, and crash recovery. They preserve canonical `SandState`, active classification, active-session start UTC, simulation UTC, periodic accumulator remainders, and one recovery target UTC.

Recovery follows:

1. claim and validate evidence;
2. persist a fixed recovery target;
3. restore checkpoint topology and engine metadata directly;
4. calculate due mass and remainders with checked integer arithmetic;
5. append missed mass as compressed pending runs;
6. publish recovered authority;
7. retain or replace checkpoint evidence according to commit state.

Missed physics frames are counted but never replayed. Recovery never installs a relaxed replacement topology. Work is independent of detached duration apart from validation and compact run changes.

SQLite publishes recovered canonical sediment, active-session continuity, the current typed daily contribution, and checkpoint state atomically. Committed evidence remains reclaimable until a fresh pending checkpoint replaces it. After successful recovery, every operational day touched by canonical session slices is reconciled.

Legacy-file authority persists a fixed recovery target and committed marker so retry deterministically overwrites from the preserved base instead of adding duplicate mass.

Normal shutdown may retire pending or committed evidence. Recovering or quarantined evidence remains protected. Runtime checkpoints are refused while mutations are queued; old mutation-bearing checkpoints fail closed because no stable cross-authority mutation receipt exists.

## Snapshot identity

A `SedimentSnapshot` envelope records:

- semantic kind;
- optional operational day;
- source revision;
- provenance;
- idle-inclusion policy;
- reconstruction status;
- canonical `SandState`.

Accepted kinds are:

- `CumulativeCheckpoint` — authentic canonical sediment at a capture point;
- `DailyContribution` — mass attributed to exactly one operational day;
- `DerivedPreview` — deterministic ledger reconstruction for viewing only.

These kinds are not interchangeable. Historical bare daily payloads are cumulative legacy evidence, not daily contributions.

## Immutable historical viewing

Historical viewing is projection-only:

- the snapshot envelope and `SandState` remain immutable;
- rendering restores a clone into a fresh viewport engine;
- physics `update()` is never called;
- repeated rendering at the same viewport is deterministic;
- cache identity includes the serialized artifact and viewport;
- the report UI exposes kind, reconstruction status, and idle policy;
- viewing never writes or deletes persistence.

If a persisted daily contribution is absent, incompatible, or stale, the report uses an in-memory `DerivedPreview`. A preview never becomes authority merely by being viewed.

## Authoritative daily contributions

Persisted historical daily sediment is a typed `DailyContribution` derived from exact operational-day session slices, including the active provisional slice when applicable.

The builder:

- includes idle explicitly and deterministically;
- orders slices by chronology and stable session identity;
- conserves every represented second;
- places up to canonical grid capacity and preserves overflow as compressed pending runs;
- records `SessionLedger` provenance and reconstruction status;
- calculates a source revision from day, grid dimensions, quantum, idle policy, category identity, elapsed seconds, slice endpoints, and session identity.

Description text is deliberately absent from the revision because it does not change sediment mass or chronology.

A persisted artifact is trusted only when snapshot schema, kind, operational day, and source revision match the current ledger-derived artifact. Otherwise the report uses a derived preview and autosave or mutation reconciliation replaces the stale authoritative contribution.

## Mutation and recovery reconciliation

Daily contribution reconciliation occurs at autosave, full-state flush, checkpoint recovery completion, and relevant session mutation boundaries.

- Deleting a canonical session captures every operational day touched by its exact overlap slices and reconciles each day after deletion.
- Description-only edits leave source revision unchanged and do not trigger sediment invalidation.
- Future category, chronology, or duration mutation must reconcile every before/after affected operational day.
- Recovery completion reconciles all days represented by completed and active canonical slices, including multi-day detached intervals.

## Legacy evidence disposition

SQLite schema version 6 adds the distinct `daily-contribution` snapshot kind. Migration from version 5 recreates the constrained table without altering existing rows, IDs, formation identities, timestamps, or `legacy_import_id` links.

Historical SQLite rows with `snapshot_kind = 'daily'` remain untouched evidence. New authority reads, writes, replaces, and deletes only `daily-contribution` rows.

Legacy-file authority writes `YYYY-MM-DD.contribution.json`. Historical `YYYY-MM-DD.json` files remain untouched evidence and are never read as authoritative daily contributions.

This is an archive-in-place disposition: legacy cumulative artifacts are preserved, named by their old format, and excluded from the new authority path. No silent reinterpretation or destructive migration occurs.

## Certification

SEDIMENT-001 is certified through PRs #50–#55.

The final D2 implementation and multi-day recovery correction passed:

- formatting;
- strict Clippy with all targets/features and warnings denied;
- 161 unit tests;
- 9 CLI lifecycle tests;
- 6 configuration-authority tests;
- 1 report-help regression test;
- 12 SQLite/TUI process tests;
- 2 temporal-authority tests;
- doc tests.

Focused proofs cover compressed billion-grain mass, immutable rendering, revision reuse and stale fallback, mass conservation beyond physical capacity, SQLite schema 5→6 migration with legacy-row retention, distinct file custody paths, typed SQLite round-trip, fault rollback, cross-day session deletion reconciliation, and multi-day recovery reconciliation.

## Remaining non-authority

SEDIMENT-001 is complete. The following remain separate future design questions rather than sediment defects:

- zoom, compression, panning, minimaps, or explicit canonical-canvas migration;
- final vertical chronology semantics beyond the accepted bottom-aligned viewport projection;
- safe queued-mutation checkpoint replay if stable cross-authority receipts are later defined;
- configurable temporal quantum and its migration rules.
