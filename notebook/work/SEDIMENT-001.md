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

Issues: #16, #26.

- establish explicit terminal-cell and dot-grid dimension vocabulary;
- render one Braille character per terminal cell;
- scan all ingress columns before reporting blockage;
- retain blocked grains in a category-preserving pending reservoir;
- persist and restore pending logical mass;
- certify arbitrary ingress occupancy, full blockage, and exact output dimensions.

### SEDIMENT-001B — logical canvas and viewport projection

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

Implement SEDIMENT-001A only. Do not solve resize, recovery, or snapshot semantics through temporary engine behavior that would constrain their later authority incorrectly.
