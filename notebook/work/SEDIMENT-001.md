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

Make sediment an accountable projection of elapsed time whose logical mass is never silently created, discarded, or reclassified by spawn collisions, viewport geometry, recovery, or historical viewing.

Chronological ledger truth remains the exact time authority. Sediment preserves accountable visual history with its own explicit topology and projection obligations.

## Required invariants

- Every due grain exists exactly once as a placed or pending logical grain.
- Total and per-category logical mass survive resize, persistence, restore, and interrupted recovery.
- Terminal-cell dimensions and Braille-dot grid dimensions are distinct.
- Viewport changes do not mutate canonical sediment merely to fit the screen.
- Existing topology is not globally relaxed as a side effect of reopening or viewing history.
- Historical snapshot kinds are explicit and immutable while viewed.

## Certified sequence

### SEDIMENT-001A — dimensions and ingress

Status: implemented and certified in PR #50.
Issues completed: #16, #26.

- explicit terminal-cell and dot-grid dimension vocabulary;
- exactly one Braille character per terminal cell;
- complete randomized ingress scan before physical blockage;
- category-preserving pending reservoir for blocked grains;
- placed-plus-pending logical mass accounting;
- backward-compatible persistence and restore of pending mass;
- coverage for arbitrary ingress occupancy, full blockage, round-trip identity, and exact output dimensions.

Accepted authority is recorded in `docs/SEDIMENT_AUTHORITY.md` and STRATA-D023 through STRATA-D024.

### SEDIMENT-001B — logical canvas and viewport projection

Status: implemented and certified in PR #51.
Issue completed: #7.

- persisted logical dot grid owns canonical dimensions and coordinates;
- terminal dimensions are presentation-only viewport state;
- shrink crops without deleting or repacking hidden grains;
- expansion pads without stretching or relocating canonical grains;
- projection is horizontally centered and bottom-aligned;
- resize never invokes gravity, ingress placement, or topology rewriting;
- restore preserves stored canonical dimensions on any opening viewport;
- shrink/expand and repeated oscillation preserve exact `SandState`;
- the destructive edge-band resize module is removed.

Accepted authority is recorded in `docs/SEDIMENT_AUTHORITY.md` and STRATA-D025.

### SEDIMENT-001C1 — compressed recovery mass

Status: implemented and certified in PR #52.
Issue advanced: #6; not closed.

- pending mass is represented as ordered category/count runs;
- adjacent same-category additions merge while transitions preserve FIFO order;
- bulk addition and storage are independent of the number of grains represented;
- ingress flush work is bounded by currently free columns;
- `SandState` schema version 2 serializes runs and migrates version 1 pending vectors;
- overflow fails visibly;
- periodic event counts and remainders use checked integer arithmetic without replay;
- a billion blocked grains are certified as one run.

Accepted authority is recorded in `docs/SEDIMENT_AUTHORITY.md` and STRATA-D026 through STRATA-D027.

### SEDIMENT-001C2 — durable bounded detached recovery

Status: next.
Issue: #6.

- claim and validate checkpoint evidence before applying recovery;
- restore committed canonical topology directly;
- calculate detached elapsed contribution once with exact periodic arithmetic;
- add missed category mass through compressed runs rather than frame replay;
- preserve topology instead of installing a relaxed catch-up replacement;
- retain checkpoint evidence until recovered sediment and session state commit together;
- certify short gaps, extreme gaps, repeated reopen, interrupted commit, stale/invalid checkpoints, and exact mass without duplication or loss.

### SEDIMENT-001D — snapshot identity

Issue: #18.

- distinguish cumulative checkpoints, daily contributions, and derived previews;
- make historical viewing immutable;
- record snapshot kind and source provenance;
- invalidate or rebuild the correct artifacts after session mutation;
- establish deterministic idle inclusion and reconstructed-preview marking.

## Current edge

Implement SEDIMENT-001C2. The engine can now represent and calculate arbitrarily large detached contributions without linear allocation or replay. Integrate that authority with checkpoint claiming, validation, atomic SQLite commit, legacy-file custody, and lifecycle semantics so issue #6 can close without weakening canonical topology.
