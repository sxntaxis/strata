---
id: SEDIMENT-001
kind: work
state: completed
authority: accepted
created: 2026-08-02
updated: 2026-08-02
---

# SEDIMENT-001 — conserved sediment authority

## Objective

Make sediment an accountable projection of elapsed time whose logical mass is never silently created, discarded, reclassified, or mutated by ingress collisions, viewport geometry, recovery, persistence, or historical viewing.

Chronological ledger truth remains the exact time authority. Sediment preserves accountable visual history with explicit mass, topology, recovery, snapshot, and projection obligations.

## Certified invariants

- Every due grain exists exactly once as placed or pending logical mass.
- Total and per-category mass survive blockage, resize, persistence, restore, and interrupted recovery.
- Terminal-cell and Braille-dot dimensions are distinct.
- Viewport changes do not mutate canonical sediment.
- Recovery is bounded, preserves topology, and never replays missed physics.
- Unresolved recovery evidence is retained and fails closed.
- Snapshot kinds are explicit and non-interchangeable.
- Historical viewing is immutable and projection-only.
- Persisted daily contributions are derived from exact ledger slices and trusted only on revision match.
- Relevant mutation and recovery reconcile every affected operational day.
- Legacy cumulative daily artifacts remain explicit archive-in-place evidence.

## Completed sequence

### SEDIMENT-001A — dimensions and ingress

PR #50. Issues #16 and #26.

- explicit geometry units;
- exact Braille viewport dimensions;
- complete ingress scanning;
- durable category-preserving pending mass.

Accepted authority: STRATA-D023 through STRATA-D024.

### SEDIMENT-001B — logical canvas and viewport projection

PR #51. Issue #7.

- canonical logical grid independent of terminal size;
- projection-only resize;
- hidden-grain preservation;
- direct canonical restore;
- destructive resize behavior removed.

Accepted authority: STRATA-D025.

### SEDIMENT-001C1 — compressed recovery mass

PR #52. Issue #6 prerequisite.

- ordered category/count pending runs;
- bounded bulk addition and storage;
- `SandState` v2 migration;
- exact periodic arithmetic without replay.

Accepted authority: STRATA-D026 through STRATA-D027.

### SEDIMENT-001C2 — durable bounded recovery

PR #53. Issue #6.

- autosave/detach/closure/crash checkpoints;
- claimed evidence and fixed recovery target;
- topology-preserving compressed recovered mass;
- no missed-physics replay;
- atomic/reclaimable SQLite publication;
- deterministic legacy retry markers;
- protected unresolved evidence.

Accepted authority: STRATA-D028 through STRATA-D029.

### SEDIMENT-001D1 — snapshot identity and immutable viewing

PR #54. Issue #18 prerequisite.

- typed cumulative, daily, and derived artifacts;
- explicit day, revision, provenance, idle policy, reconstruction status, and state;
- cumulative evidence cannot substitute for daily contributions;
- deterministic read-only preview fallback;
- immutable historical rendering.

Accepted authority: STRATA-D030 through STRATA-D031.

### SEDIMENT-001D2 — daily contribution authority

PR #55. Issue #18.

- typed ledger-derived daily persistence;
- exact mass conservation including pending overflow;
- revision validation and stale fallback;
- SQLite schema 6;
- distinct file contribution paths;
- cross-boundary deletion invalidation;
- multi-day recovery reconciliation;
- archive-in-place legacy custody.

Accepted authority: STRATA-D032 through STRATA-D033.

## Closure

SEDIMENT-001 is complete. Issues #6, #7, #16, #18, and #26 are satisfied by one coherent conservation model.

The next program is INTERACTION-001 for issues #19, #20, and #24.
