# Sediment authority

Status: partially implemented and certified
Program: SEDIMENT-001
Current completed unit: SEDIMENT-001D1
Issues completed: #6, #7, #16, #26
Last reviewed: 2026-08-02

## Purpose

Sediment is accountable visual history derived from elapsed time. Chronological sessions remain the exact time authority, while sediment preserves explicit mass, category, topology, recovery, snapshot, and projection obligations.

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

Runtime checkpoints cover detach, terminal closure, crash, and periodic autosave recovery. They preserve canonical `SandState`, active classification, active-session start UTC, simulation UTC, accumulator remainders, and one recovery-attempt target UTC.

A checkpoint is claimed before derivation. SQLite changes `pending` or `committed` evidence to `recovering`; legacy-file authority persists the recovery target in the checkpoint payload. Invalid or unsupported evidence remains present and startup fails closed.

Checkpoint validation rejects unsupported schemas, future or non-monotonic timestamps, missing or mismatched identities, unavailable category identities, duplicate or out-of-bounds coordinates, invalid accumulators, arithmetic overflow, and legacy checkpoints containing queued mutations.

New runtime checkpoints are not written while queued mutations exist. Old checkpoints that already contain queued mutations are retained and rejected because those mutations do not have one stable cross-authority receipt identity.

## Bounded detached recovery

Recovery restores the checkpoint canvas and metadata directly. It calculates elapsed time and exact periodic remainders, appends missed category mass as compressed pending runs, preserves all checkpoint coordinates and engine metadata, and never replays missed physics or installs a relaxed topology.

SQLite publishes recovered sediment, daily snapshot, active-session continuity, and checkpoint status atomically. Committed evidence remains reclaimable until a fresh pending checkpoint replaces it. Legacy-file authority persists a fixed recovery target and committed marker so retry deterministically overwrites from the preserved base rather than adding duplicate mass.

Normal shutdown may retire `pending` or `committed` checkpoint evidence. It refuses to clear `recovering` or `quarantined` evidence.

## Snapshot identity

A `SedimentSnapshot` envelope gives historical artifacts an explicit semantic identity. Its schema records:

- snapshot kind;
- optional operational day;
- source revision;
- provenance;
- idle-inclusion policy;
- reconstruction status;
- canonical `SandState` payload.

The accepted kinds are:

- `CumulativeCheckpoint` — authentic canonical sediment as of a capture point;
- `DailyContribution` — sediment mass attributed to exactly one operational day;
- `DerivedPreview` — deterministic visualization reconstructed from chronological ledger truth for viewing only.

These kinds are not interchangeable. In particular, cumulative sediment captured under a historical `daily` storage key is classified as `CumulativeCheckpoint` with `LegacyDailyRow` provenance. It cannot silently satisfy a request for one day's contribution.

When no compatible daily contribution exists, the report builds an in-memory `DerivedPreview` from the selected day's canonical session slices. The preview records `SessionLedger` provenance, `reconstructed = true`, and an explicit idle policy. Its source revision changes when the ordered day-owned chronology changes.

SEDIMENT-001D1 does not overwrite, delete, or reclassify persisted legacy daily rows. Their final custody and authoritative daily-contribution replacement belong to SEDIMENT-001D2.

## Immutable historical viewing

Historical viewing is a projection-only operation:

- the snapshot envelope and `SandState` remain immutable;
- rendering creates a fresh viewport engine and restores the artifact into it;
- only presentation dimensions adapt to the current terminal;
- physics `update()` is never called;
- repeated rendering at the same viewport returns identical lines;
- render cache identity includes the serialized artifact and viewport, not merely physical grain-vector length;
- report UI labels the artifact as cumulative, daily, or derived and marks reconstruction and idle policy visibly;
- viewing or rebuilding an in-memory preview performs no persistence write or deletion.

Report viewing therefore cannot become a competing mutable sediment authority.

## Certification

### SEDIMENT-001A

Proves complete ingress scanning, blocked-grain conservation, pending-state round-trip, exact output dimensions, and explicit geometry units.

### SEDIMENT-001B

Proves exact logical-state preservation across resize, hidden-grain reappearance, direct canonical restore, and removal of destructive resize behavior.

### SEDIMENT-001C1

Proves billion-grain compression, ordered run merging, bounded ingress work, exact category removal, schema migration, overflow refusal, and long-duration periodic arithmetic without replay.

### SEDIMENT-001C2

Proves bounded topology-preserving recovery, exact short/extreme gap mass, reclaimable atomic evidence, safe normal shutdown retirement, and protection of unresolved recovery evidence.

### SEDIMENT-001D1

Proves:

- all three snapshot kinds are distinct and serializable;
- legacy bare daily `SandState` is classified as cumulative legacy evidence;
- cumulative evidence cannot substitute for a daily contribution;
- repeated historical rendering is deterministic and leaves coordinates, pending runs, frame count, sweep direction, and RNG unchanged;
- source revision changes with chronology material;
- incompatible or missing daily artifacts fall back to a marked derived preview without persistence mutation;
- formatting, strict Clippy, and the complete all-features suite pass.

## Remaining SEDIMENT-001 authority

SEDIMENT-001D2 / issue #18 remains responsible for:

- authoritative persistence of `DailyContribution` envelopes;
- source-revision comparison and stale-artifact rebuilding;
- correct edit/delete invalidation across all affected operational days;
- explicit archive, migration, or removal disposition for legacy cumulative daily rows;
- certification that persisted daily artifacts and in-memory previews cannot diverge silently.

Until D2 is certified, snapshot identity and immutable viewing are authoritative, but daily-contribution persistence and mutation invalidation are not complete.
