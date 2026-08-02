---
id: SEDIMENT-001
kind: work
state: active
authority: working
created: 2026-08-02
updated: 2026-08-02
---

# SEDIMENT-001 — conserved sediment authority

## Objective

Make sediment an accountable projection of elapsed time whose logical mass is never silently created, discarded, reclassified, or mutated by ingress collisions, viewport geometry, recovery, or historical viewing.

Chronological ledger truth remains the exact time authority. Sediment preserves accountable visual history with explicit mass, topology, recovery, snapshot, and projection obligations.

## Required invariants

- Every due grain exists exactly once as placed or pending logical mass.
- Total and per-category mass survive resize, persistence, restore, and interrupted recovery.
- Terminal-cell and Braille-dot dimensions are distinct.
- Viewport changes do not mutate canonical sediment.
- Recovery does not replay unbounded physics or relax checkpoint topology.
- Unresolved recovery evidence is retained and fails closed.
- Snapshot kinds are explicit and non-interchangeable.
- Historical viewing is immutable and projection-only.
- Persisted daily contributions eventually remain revision-matched to ledger truth.

## Certified sequence

### SEDIMENT-001A — dimensions and ingress

Status: implemented and certified in PR #50.
Issues completed: #16, #26.

- explicit cell/dot dimension vocabulary;
- exact Braille output dimensions;
- complete ingress scanning;
- durable category-preserving pending mass.

Accepted authority: STRATA-D023 through STRATA-D024.

### SEDIMENT-001B — logical canvas and viewport projection

Status: implemented and certified in PR #51.
Issue completed: #7.

- canonical logical grid independent of terminal size;
- projection-only resize;
- hidden-grain preservation;
- direct canonical restore;
- destructive resize module removed.

Accepted authority: STRATA-D025.

### SEDIMENT-001C1 — compressed recovery mass

Status: implemented and certified in PR #52.
Issue advanced: #6.

- ordered category/count pending runs;
- bounded bulk addition and storage;
- `SandState` v2 migration;
- exact periodic arithmetic without replay.

Accepted authority: STRATA-D026 through STRATA-D027.

### SEDIMENT-001C2 — durable bounded detached recovery

Status: implemented and certified in PR #53.
Issue completed: #6.

- autosave/detach/closure/crash checkpoints;
- claimed evidence and persisted recovery target;
- topology-preserving compressed recovered mass;
- no missed-physics replay;
- atomic/reclaimable SQLite publication;
- deterministic legacy retry markers;
- protected unresolved evidence.

Accepted authority: STRATA-D028 through STRATA-D029.

### SEDIMENT-001D1 — snapshot identity and immutable viewing

Status: implemented and certified in PR #54.
Issue advanced: #18; not closed.

- typed `CumulativeCheckpoint`, `DailyContribution`, and `DerivedPreview` artifacts;
- explicit day, revision, provenance, idle policy, reconstruction status, and `SandState`;
- legacy bare daily payloads classified as cumulative evidence;
- cumulative evidence cannot substitute for daily contributions;
- deterministic ledger-derived preview fallback;
- immutable historical rendering with no physics or persistence mutation;
- visible artifact status in the report UI.

Accepted authority: STRATA-D030 through STRATA-D031.

### SEDIMENT-001D2 — daily contribution persistence and invalidation

Status: next.
Issue: #18.

- persist typed `DailyContribution` envelopes rather than cumulative live state under daily keys;
- compare source revisions before trusting a persisted artifact;
- rebuild or invalidate every operational day affected by session edits and deletions;
- preserve deterministic idle inclusion;
- archive, migrate, or remove legacy cumulative daily rows with explicit provenance;
- certify SQLite and legacy-file parity;
- close issue #18 without making previews a competing authority.

## Current edge

Implement SEDIMENT-001D2. Snapshot meaning and immutable viewing are now explicit. The remaining sediment gap is persistence: daily artifacts must be true contributions whose source revision matches canonical ledger chronology, with complete mutation invalidation and explicit legacy-row disposition.
