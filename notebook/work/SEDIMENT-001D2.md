---
id: SEDIMENT-001D2
kind: work
state: completed
authority: accepted
created: 2026-08-02
updated: 2026-08-02
---

# SEDIMENT-001D2 — daily contribution persistence and invalidation

## Objective

Persist historical daily sediment as truthful typed contributions derived from canonical ledger slices, not as cumulative live-state copies.

## Accepted contract

- Persist `DailyContribution` envelopes under distinct daily authority keys.
- Derive payload and source revision from exact operational-day session slices.
- Include idle explicitly and deterministically.
- Conserve every second, preserving overflow as compressed pending runs.
- Trust persisted artifacts only when schema, kind, day, and revision match current ledger truth.
- Use in-memory `DerivedPreview` fallback for missing or stale authority without writing during report view.
- Reconcile authoritative contributions during autosave, full-state flush, relevant mutation, and recovery completion.
- Reconcile every operational day touched by a deleted or recovered cross-boundary session.
- Exclude descriptions from source revision because description-only edits do not change sediment mass.
- Preserve historical cumulative daily artifacts as archive-in-place evidence.

## Persistence authority

### SQLite

- Schema version 6 adds `snapshot_kind = 'daily-contribution'`.
- Version 5→6 migration recreates the constrained table while preserving rows, IDs, formation identity, timestamps, payloads, and legacy-import links.
- New reads/writes/deletes address only `daily-contribution` rows.
- Historical `daily` rows remain untouched evidence.
- Checkpoint recovery atomically publishes canonical sediment plus the current typed daily contribution, then reconciles every represented operational day.

### Legacy files

- New authority uses `YYYY-MM-DD.contribution.json`.
- Historical `YYYY-MM-DD.json` files remain untouched evidence.
- File and SQLite paths serialize the same `SedimentSnapshot` envelope.

## Mutation rules

- Canonical session deletion captures every operational day from exact session overlap slices before deletion and reconciles each day afterward.
- Description-only edits leave the source revision unchanged.
- Future chronology, category, duration, or policy mutation must reconcile every before/after affected day.
- Recovery completion reconciles all days represented by completed and active canonical slices, including multi-day detached intervals.

## Certified proofs

- cumulative live state is no longer written under daily authority keys;
- typed daily contributions round-trip with kind, day, revision, provenance, idle policy, reconstruction status, and `SandState`;
- matching revision reuses persisted authority;
- stale revision falls back to the exact derived artifact;
- physical capacity overflow remains conserved pending mass;
- description-independent revision behavior is explicit;
- SQLite schema 5→6 preserves legacy daily evidence and admits typed contributions;
- legacy and typed SQLite rows coexist without substitution;
- file contribution paths are distinct from historical daily paths;
- SQLite fault certification retains rollback/recovery guarantees;
- cross-boundary deletion reconciles every touched day;
- recovery completion reconciles every represented day;
- formatting, strict Clippy, 161 unit tests, 9 CLI lifecycle tests, 6 configuration tests, 1 report-help test, 12 SQLite/TUI process tests, 2 temporal tests, and doc tests pass.

## Durable authority

- `docs/SEDIMENT_AUTHORITY.md` closes the sediment program.
- `docs/ARCHITECTURE.md` assigns typed daily construction and reconciliation boundaries.
- STRATA-D032 and STRATA-D033 constrain revision acceptance, invalidation, and legacy custody.
- `notebook/work/SEDIMENT-001.md` is complete.
- `notebook/NOW.md` advances to INTERACTION-001.

## Result

Issue #18 is complete. Snapshot identity, immutable viewing, authoritative daily persistence, revision comparison, mutation invalidation, multi-day recovery reconciliation, and legacy evidence disposition now form one certified contract.
