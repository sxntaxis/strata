# Sediment authority

Status: partially implemented and certified
Program: SEDIMENT-001
Current completed unit: SEDIMENT-001C2
Issues completed: #6, #7, #16, #26
Last reviewed: 2026-08-02

## Purpose

Sediment is accountable visual history derived from elapsed time. Chronological sessions remain the exact time authority, while sediment preserves its own explicit mass, category, topology, recovery, and projection obligations.

The SEDIMENT-001 program covers ingress, viewport projection, detached recovery, and historical snapshot identity. This document records accepted sediment authority as each bounded unit becomes certified.

## Logical mass

A due grain exists exactly once as either:

- a placed physical grain in the logical dot grid; or
- a pending grain waiting for ordinary live ingress.

Physical blockage is not permission to discard elapsed-time mass. `grain_count` represents placed plus pending logical mass. Pending grains retain category identity and FIFO category order.

A newly due live grain enters the pending reservoir first. The engine chooses randomized free ingress columns and performs no more placement work than the number of currently free columns. Any unplaced remainder stays pending without creating another grain.

Clearing all sediment clears both placed and pending mass. Category-specific clearing and removal apply to both forms. Unknown category identities encountered during ordinary non-recovery restore follow the established explicit normalization to idle rather than disappearing. Checkpoint recovery is stricter and refuses unavailable identities so evidence is not silently reclassified.

## Compressed pending mass

Pending mass is stored as ordered category/count runs rather than one category ID per grain.

- Adjacent additions for the same category merge into one run.
- Category transitions remain ordered, preserving FIFO category chronology.
- Adding an arbitrarily large count performs constant work in the count itself.
- Live ingress flushing is bounded by current free ingress columns, not total pending mass.
- Snapshot size is proportional to physical grains plus category-run changes rather than blocked elapsed seconds.
- Logical and run-count overflow is rejected rather than wrapped or silently saturated.

This representation makes bounded detached recovery possible. A billion missed grains can exist as one run instead of a billion allocations or replay steps.

## Dimension units

Terminal geometry and Braille-dot geometry are distinct units:

- `cell_width` and `cell_height` are drawable terminal-cell viewport dimensions;
- `grid_width_dots` and `grid_height_dots` are canonical logical-canvas dimensions;
- one terminal cell projects `dot_width × dot_height` logical dots.

Rendering emits exactly one Braille character per drawable terminal cell. Simulation, persistence, snapshots, and capacity calculations use canonical dot-grid dimensions. Callers must not infer or compare these units through ambiguous `width` or `height` fields.

## Canonical logical canvas

The persisted logical dot grid owns sediment topology. The current terminal does not own storage dimensions.

For a new empty profile, the initial drawable viewport seeds the canonical canvas once. After that:

- `resize(width, height)` changes only terminal-cell viewport dimensions;
- canonical grid dimensions, coordinates, category neighborhoods, pending order, frame count, sweep direction, and RNG state remain unchanged;
- shrinking crops the visible projection without removing or repacking hidden grains;
- expanding pads the visible projection without stretching or relocating grains;
- projection is horizontally centered and bottom-aligned;
- restoring persisted state installs its canonical dimensions and coordinates directly;
- resize never invokes gravity, ingress placement, band packing, overflow insertion, or another canonical mutation.

The former destructive resize helper and edge-band policy are removed. Terminal oscillation with no elapsed time is exactly idempotent at the `SandState` level.

Zoom, compression, panning, minimaps, and explicit canonical-canvas migration remain outside the accepted contract. They cannot silently mutate accepted sediment history.

## Persistence compatibility

`SandState` schema version 2 stores pending category/count runs.

- Version 1 states with `pending_grains` migrate deterministically into adjacent compressed runs.
- Older JSON without either pending field loads as an empty pending reservoir.
- Version 2 writes `pending_runs` and leaves the legacy vector empty.
- Empty pending collections are omitted during serialization.
- Zero-count runs contribute no mass.
- Unknown pending category IDs normalize to idle during ordinary restore.
- Storage, SQLite state, runtime checkpoints, report previews, and derived visual projections use the same state contract.

Canonical `grid_width` and `grid_height` restore as persisted rather than adapting to the opening viewport. Viewport dimensions remain runtime presentation state.

## Exact periodic arithmetic

Periodic event counts and accumulator remainders use checked integer nanosecond arithmetic. A long elapsed interval is divided by its period directly; it is not advanced through one iteration per missed event. Zero periods, full-or-larger accumulator values, duration overflow, and unrepresentable counts fail visibly.

## Runtime checkpoint lifecycle

Runtime checkpoints cover detach, terminal closure, crash, and periodic autosave recovery. They preserve:

- canonical `SandState`;
- active category and description;
- active-session start UTC;
- simulation UTC;
- spawn and physics accumulator remainders;
- one recovery-attempt target UTC when recovery is in flight.

A checkpoint is claimed before derivation. SQLite changes `pending` or `committed` evidence to `recovering`; legacy-file authority persists the recovery target in the checkpoint payload. Invalid or unsupported evidence remains present and startup fails closed.

Checkpoint validation rejects:

- unsupported checkpoint or sediment schemas;
- future or non-monotonic timestamps;
- missing or mismatched active-session identity;
- unavailable active, placed, or pending category identities;
- duplicate or out-of-bounds grain coordinates;
- invalid periodic accumulators;
- arithmetic overflow;
- legacy checkpoints containing queued mutations.

New runtime checkpoints are not written while queued mutations exist. Old checkpoints that already contain queued mutations are retained and rejected because those mutations do not have one stable cross-authority receipt identity. SEDIMENT-001C2 does not claim to replay them safely.

## Bounded detached recovery

Recovery restores the checkpoint canvas and metadata directly. It then:

1. calculates elapsed time to a persisted recovery target;
2. calculates exact due spawn count and accumulator remainders;
3. appends missed category mass as compressed pending runs;
4. preserves all pre-checkpoint coordinates, frame count, sweep direction, and RNG state;
5. records skipped physics-event count for proof, but does not replay those frames or install a relaxed topology;
6. resumes ordinary live simulation only after recovered state is durably published.

Work is proportional to validation input and pending-run changes, not detached duration. Extreme gaps therefore remain bounded.

SQLite publishes recovered sediment, daily snapshot, active-session continuity, and checkpoint status in its existing atomic recovery transaction. `committed` evidence remains reclaimable after interruption; a successful startup replaces it with a fresh `pending` runtime checkpoint. Reclaiming committed evidence re-derives from its preserved base to the new startup time and overwrites the authoritative recovered state, so elapsed time advances without duplicate mass.

Legacy-file authority writes the fixed recovery target before derivation, atomically publishes current state and daily snapshot, and marks checkpoint evidence committed. If interruption occurs before a fresh checkpoint replaces that marker, reopening re-derives from the preserved base and deterministically overwrites the published state.

Normal shutdown may retire `pending` or `committed` checkpoint evidence. It refuses to clear `recovering` or `quarantined` evidence.

Successful bounded recovery creates no synthetic replay backlog and performs no catch-up topology replacement.

## Certification for SEDIMENT-001A

SEDIMENT-001A proves complete ingress scanning, blocked-grain conservation, pending-state round-trip, exact output dimensions, and propagation of explicit geometry units.

## Certification for SEDIMENT-001B

SEDIMENT-001B proves exact logical-state preservation across shrink/expand and repeated oscillation, hidden-grain reappearance, direct canonical restore, and removal of destructive resize behavior.

## Certification for SEDIMENT-001C1

SEDIMENT-001C1 proves billion-grain compression, ordered run merging, bounded ingress work, exact category removal, schema migration, overflow refusal, and long-duration periodic arithmetic without replay.

## Certification for SEDIMENT-001C2

SEDIMENT-001C2 proves:

- exact short-gap mass and accumulator remainder;
- billion-second recovery as one compressed run;
- preservation of checkpoint coordinates and engine metadata;
- malformed-state and invalid-period refusal;
- backward-compatible checkpoint schema fields;
- reclaimable committed SQLite evidence and atomic publication;
- safe retirement of `pending` checkpoints on normal shutdown;
- protection of `recovering` evidence from shutdown clearing;
- successful normal TUI exit and post-commit reload retry;
- formatting, strict Clippy, 153 unit tests, 9 CLI lifecycle tests, 6 configuration tests, 1 report-help test, 12 SQLite/TUI process tests, 2 temporal tests, and doc tests.

## Remaining SEDIMENT-001 authority

The remaining contract is:

- explicit immutable snapshot kinds and provenance — SEDIMENT-001D / issue #18.

Until SEDIMENT-001D is certified, historical snapshot meaning remains the final incomplete part of sediment authority.
