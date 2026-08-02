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
- Recovery does not replay unbounded physics or relax checkpoint topology.
- Unresolved recovery evidence is retained and fails closed.
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
- backward-compatible persistence and restore of pending mass.

Accepted authority: STRATA-D023 through STRATA-D024.

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
- the destructive edge-band resize module is removed.

Accepted authority: STRATA-D025.

### SEDIMENT-001C1 — compressed recovery mass

Status: implemented and certified in PR #52.
Issue advanced: #6.

- pending mass is represented as ordered category/count runs;
- adjacent same-category additions merge while transitions preserve FIFO order;
- bulk addition and storage are independent of represented grain count;
- ingress flush work is bounded by currently free columns;
- `SandState` schema version 2 serializes runs and migrates version 1 vectors;
- overflow fails visibly;
- periodic event counts and remainders use checked integer arithmetic without replay.

Accepted authority: STRATA-D026 through STRATA-D027.

### SEDIMENT-001C2 — durable bounded detached recovery

Status: implemented and certified in PR #53.
Issue completed: #6.

- runtime checkpoints cover autosave, detach, terminal closure, and crash recovery;
- evidence is claimed and a target is persisted before publication;
- canonical topology and engine metadata restore directly;
- detached elapsed mass is added as compressed pending runs;
- missed physics frames are not replayed;
- SQLite recovery publication is atomic and committed evidence remains reclaimable;
- legacy-file recovery uses deterministic target and committed markers;
- normal shutdown clears only pending or committed evidence;
- recovering and quarantined evidence remains protected;
- queued-mutation checkpoints fail closed because stable cross-authority mutation receipts do not exist;
- repeated reopen and interrupted publication do not duplicate or lose mass.

Accepted authority: STRATA-D028 through STRATA-D029.

### SEDIMENT-001D — snapshot identity

Status: next.
Issue: #18.

- distinguish cumulative checkpoints, daily contributions, and derived previews;
- make historical viewing immutable;
- record snapshot kind and source provenance;
- invalidate or rebuild the correct artifacts after session mutation;
- establish deterministic idle inclusion and reconstructed-preview marking;
- prevent report viewing from becoming a competing mutable sediment authority.

## Current edge

Implement SEDIMENT-001D. Mass, topology, viewport behavior, and runtime recovery are now conserved. Historical snapshot meaning is the remaining sediment authority gap: each artifact must declare whether it is a cumulative checkpoint, a daily contribution, or a derived preview, and historical viewing must remain immutable.
