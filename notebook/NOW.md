---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-02
authority: working
summary: SEDIMENT-001 is complete; sediment mass, topology, recovery, snapshots, and daily persistence are conserved.
next: Implement INTERACTION-001 for issues #19, #20, and #24.
---

# NOW — Strata

## Current phase

The SQLite migration program, AUTHORITY-001, TEMPORAL-001, TEMPORAL-002, DOMAIN-001, REPORT-001, and the complete SEDIMENT-001 sequence are finished.

Strata now has:

- durable fail-closed persistence;
- explicit monotonic/UTC/fixed-offset time authority;
- canonical project/category/session identity;
- truthful deterministic report and export projections;
- lossless sediment ingress;
- viewport-independent canonical topology;
- compressed logical mass;
- bounded topology-preserving recovery;
- explicit snapshot identity and immutable historical viewing;
- revision-matched typed daily contributions;
- complete cross-day mutation and recovery reconciliation;
- archive-in-place legacy snapshot custody.

The project is beginning **interaction authority**.

## Accepted product baseline

- Strata is a continuous temporal ledger and an active timer.
- Time always passes; the baseline is idle.
- Idle produces sediment but is excluded from ordinary active-time totals.
- Exact chronological history and accountable sedimentary history are complementary truths.
- One grain currently represents one elapsed second.
- Every due grain is exactly one placed or pending logical grain.
- Terminal geometry is projection state, not canonical sediment authority.
- Runtime recovery cannot replay unbounded physics or relax topology.
- Cumulative checkpoints, daily contributions, and derived previews are distinct.
- Historical viewing is immutable.
- Persisted daily contributions are trusted only when their revision matches canonical ledger slices.

## Verified technical baseline

- SQLite schema version 6 is authoritative after explicit activation.
- CLI and TUI share configuration, repository, temporal, session, recovery, and snapshot boundaries.
- Reports use exact operational-day slices and explicit provisional active state.
- JSON and ICS have deterministic ordering and authoritative UTC chronology.
- `SandState` v2 stores compressed pending runs and migrates v1 vectors.
- Resize, persistence, restore, and recovery conserve mass and topology.
- Runtime checkpoints cover autosave, detach, terminal closure, and crash recovery.
- Recovery publication is bounded, atomic/reclaimable in SQLite, deterministic in legacy-file authority, and reconciles all represented days.
- Snapshot envelopes expose kind, day, revision, provenance, idle policy, reconstruction status, and state.
- Historical rendering never advances physics or writes persistence.
- Daily contributions derive from exact canonical session slices, conserve overflow as pending mass, and include idle explicitly.
- Stale contributions fall back to in-memory derived previews until reconciliation.
- Session deletion rebuilds every touched operational day.
- Legacy SQLite `daily` rows and legacy daily JSON files remain untouched evidence.

## Completed post-migration units

- **AUTHORITY-001** — issue #21.
- **TEMPORAL-001** — issue #25.
- **TEMPORAL-002** — issues #4, #23, #27.
- **DOMAIN-001** — issues #2, #12.
- **REPORT-001** — issues #1, #3, #14, #17, #28.
- **SEDIMENT-001A** — issues #16, #26.
- **SEDIMENT-001B** — issue #7.
- **SEDIMENT-001C1/C2** — issue #6.
- **SEDIMENT-001D1/D2** — issue #18.

## Active sequence

1. **INTERACTION-001** — issues #19, #20, #24: explicit edit modes, terminal lifecycle guard, and truthful keybinding behavior.
2. Reconcile partially satisfied issues #5, #10, and #13.
3. Later domain/profile work, including issue #15 and issue #22.

## Current risks

- Interaction edit modes are not fully explicit or isolated.
- Terminal cleanup still needs one process-wide lifecycle guard.
- Keybinding configuration and runtime behavior require a final truth audit.
- Queued checkpoint mutations have no stable cross-authority receipt identity and fail closed.
- Complete profile switching/isolation remains open.

## Next

Implement **INTERACTION-001** for issues #19, #20, and #24. Establish explicit modal ownership, guarantee terminal restoration across every exit/panic path, and certify that configured bindings match actual runtime behavior without hidden global shortcuts.
