# Sediment authority

Status: partially implemented and certified
Program: SEDIMENT-001
Current completed unit: SEDIMENT-001C1
Issues completed: #7, #16, #26
Last reviewed: 2026-08-02

## Purpose

Sediment is accountable visual history derived from elapsed time. Chronological sessions remain the exact time authority, while sediment preserves its own explicit mass, category, topology, and projection obligations.

The full SEDIMENT-001 program covers ingress, viewport projection, detached recovery, and historical snapshot identity. This document records accepted sediment authority as each bounded unit becomes certified.

## Logical mass

A due grain exists exactly once as either:

- a placed physical grain in the logical dot grid; or
- a pending grain waiting for an ingress position.

Physical blockage is not permission to discard elapsed-time mass. `grain_count` therefore represents placed plus pending logical mass. Pending grains retain category identity and FIFO category order.

A newly due grain enters the pending reservoir first. The engine chooses randomized free ingress columns and performs no more placement work than the number of currently free columns. Any unplaced remainder stays pending without creating another grain.

Clearing all sediment clears both placed and pending mass. Category-specific clearing and removal apply to both forms. Unknown category identities encountered during restore follow the established explicit normalization to idle rather than disappearing.

## Compressed pending mass

Pending mass is stored as ordered category/count runs rather than one category ID per grain.

- Adjacent additions for the same category merge into one run.
- Category transitions remain ordered, preserving FIFO category chronology.
- Adding an arbitrarily large count performs constant work in the count itself, apart from placing grains into currently free ingress columns.
- Flushing is bounded by current ingress capacity, not by total pending mass.
- Snapshot size is proportional to physical grains plus category-run changes rather than blocked elapsed seconds.
- Logical and run-count overflow is rejected rather than wrapped or silently saturated.

This representation is required for bounded detached recovery. A billion missed grains can exist as one run instead of a billion allocations or replay steps.

## Dimension units

Terminal geometry and Braille-dot geometry are distinct units:

- `cell_width` and `cell_height` are drawable terminal-cell viewport dimensions;
- `grid_width_dots` and `grid_height_dots` are canonical logical-canvas dimensions;
- one terminal cell projects `dot_width × dot_height` logical dots.

Rendering iterates terminal cells and emits exactly one Braille character per drawable cell. Simulation, persistence, snapshots, and capacity calculations use canonical dot-grid dimensions. Callers must not compare, divide, or infer these units through ambiguous `width` or `height` fields.

## Canonical logical canvas

The persisted logical dot grid owns sediment topology. The current terminal does not own storage dimensions.

For a new empty profile, the initial drawable viewport seeds the canonical canvas once. After that:

- `resize(width, height)` changes only the terminal-cell viewport;
- canonical grid dimensions, coordinates, category neighborhoods, pending order, frame count, sweep direction, and RNG state remain unchanged;
- shrinking crops the visible projection without removing or repacking hidden grains;
- expanding pads the visible projection without stretching or relocating grains;
- projection is centered horizontally and bottom-aligned vertically so the settled base remains visible when possible;
- restoring persisted state installs its canonical dimensions and coordinates directly, regardless of the terminal size used to open it;
- resize never invokes gravity, ingress placement, band packing, overflow insertion, or any other canonical mutation.

The former resize helper and its edge-band policy are removed. Terminal oscillation with no elapsed time is therefore exactly idempotent at the `SandState` level.

This unit deliberately does not define zoom, compression, panning, minimaps, or explicit canonical-canvas migration. Those may be designed later, but they cannot silently mutate accepted sediment history.

## Persistence compatibility

`SandState` schema version 2 stores pending category/count runs.

- Version 1 states with `pending_grains` migrate deterministically into adjacent compressed runs.
- Older JSON without either pending field loads as an empty pending reservoir.
- Version 2 writes `pending_runs` and leaves the legacy vector empty.
- Empty pending collections are omitted during serialization.
- Zero-count runs contribute no mass.
- Unknown pending category IDs normalize to idle under the same rule as placed grains.
- Storage, SQLite state, detached checkpoints, catch-up projections, and report previews use the same state contract.

Canonical `grid_width` and `grid_height` restore as persisted rather than adapting to the opening viewport. Viewport dimensions remain runtime presentation state and are not written into canonical sediment merely because the terminal changed.

## Exact periodic arithmetic

Recovery event counts and accumulator remainders are calculated with checked integer nanosecond arithmetic. A long elapsed interval is divided by its period directly; it is not advanced through one loop iteration per missed event. Zero periods and unrepresentable counts fail visibly.

SEDIMENT-001C1 certifies this arithmetic as a prerequisite. SEDIMENT-001C2 must connect it to checkpoint lifecycle and session recovery before detached recovery is considered complete.

## Certification for SEDIMENT-001A

SEDIMENT-001A proves:

- a grain finds the only free ingress column regardless of randomized starting position;
- a fully blocked grain remains logical and later enters when ingress reopens;
- logical and category mass survive snapshot/restore with pending state;
- terminal output dimensions equal the requested cell dimensions exactly;
- named cell and dot-grid dimensions propagate through TUI rendering, catch-up projection, report previews, persistence, and fault tests.

## Certification for SEDIMENT-001B

SEDIMENT-001B proves:

- shrink → expand preserves the exact serialized `SandState`;
- repeated oscillating resizes are logically idempotent;
- total mass, per-category identity, coordinates, vertical order, local neighborhoods, pending order, and engine metadata remain unchanged;
- grains outside a smaller viewport remain hidden rather than discarded and reappear when the viewport expands;
- render output always matches current terminal-cell dimensions independently of canonical canvas size;
- restore on a differently sized viewport preserves stored dimensions and coordinates exactly;
- the destructive resize module and resize-triggered global gravity path no longer exist.

## Certification for SEDIMENT-001C1

SEDIMENT-001C1 proves:

- one billion blocked grains require one pending run;
- adjacent same-category additions merge while category transitions preserve order;
- ingress flushing performs at most one placement per currently free ingress column;
- category clearing and counted removal operate exactly across compressed runs;
- version 1 pending vectors migrate into equivalent version 2 runs;
- version 2 snapshot/restore preserves run order, counts, identity, and total mass;
- long-duration periodic calculation returns exact due counts and accumulator remainder without replay;
- formatting, strict Clippy, and the full all-features suite pass.

## Remaining SEDIMENT-001 authority

The following contracts remain open:

- integration of compressed mass with durable, retry-safe detached checkpoint recovery — SEDIMENT-001C2 / issue #6;
- explicit immutable snapshot kinds and provenance — SEDIMENT-001D / issue #18.

Until those units are certified, detached checkpoint lifecycle and historical snapshots retain their current implementation limits and must not be described as fully conserved sediment authority.
