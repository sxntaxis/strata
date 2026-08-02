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

Status: next.
Issue: #7.

- separate canonical logical sediment dimensions from current viewport dimensions;
- make shrink a non-destructive view operation;
- preserve hidden grains for later expansion;
- remove resize-triggered global gravity and repacking from canonical history;
- define topology-preservation tolerances and round-trip proofs.

### SEDIMENT-001C — bounded detached recovery

Issue: #6.

- restore committed topology directly;
- calculate elapsed contribution once;
- add missed logical mass in bounded work rather than replaying every physics frame;
- retain checkpoint evidence until recovered sediment and session state commit together;
- certify repeated reopen and interrupted recovery without duplication or loss.

### SEDIMENT-001D — snapshot identity

Issue: #18.

- distinguish cumulative checkpoints, daily contributions, and derived previews;
- make historical viewing immutable;
- record snapshot kind and source provenance;
- invalidate or rebuild the correct artifacts after session mutation;
- establish deterministic idle inclusion and reconstructed-preview marking.

## Current edge

Implement SEDIMENT-001B. The current engine still treats viewport dimensions as canonical storage and runs resize repacking/gravity. Replace that coupling without weakening the placed/pending mass authority established by 001A.
