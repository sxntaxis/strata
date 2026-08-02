# Sediment authority

Status: partially implemented and certified
Program: SEDIMENT-001
Current completed unit: SEDIMENT-001B
Issues completed: #7, #16, #26
Last reviewed: 2026-08-02

## Purpose

Sediment is accountable visual history derived from elapsed time. Chronological sessions remain the exact time authority, while sediment preserves its own explicit mass, category, topology, and projection obligations.

The full SEDIMENT-001 program covers ingress, viewport projection, detached recovery, and historical snapshot identity. This document records accepted sediment authority as each bounded unit becomes certified.

## Logical mass

A due grain exists exactly once as either:

- a placed physical grain in the logical dot grid; or
- a pending grain waiting for an ingress position.

Physical blockage is not permission to discard elapsed-time mass. `grain_count` therefore represents placed plus pending logical mass. Pending grains retain category identity, preserve FIFO order, and are persisted with `SandState`.

A newly due grain enters the pending reservoir first. The engine chooses a randomized starting column and scans the entire ingress row exactly once. It remains pending only when every ingress column is occupied. Normal simulation updates retry pending ingress without creating another grain.

Clearing all sediment clears both placed and pending mass. Category-specific clearing and removal apply to both forms. Unknown category identities encountered during restore follow the existing explicit normalization to idle rather than disappearing.

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

`SandState.pending_grains` is backward-compatible:

- older JSON without the field loads as an empty pending reservoir;
- an empty reservoir is omitted during serialization, preserving prior ordinary state shape;
- nonempty pending mass is serialized in category-preserving order;
- storage, SQLite state, detached checkpoints, catch-up projections, and report previews use the same state contract.

Canonical `grid_width` and `grid_height` are restored as persisted rather than adapted to the opening viewport. Viewport dimensions remain runtime presentation state and are not written into canonical sediment merely because the terminal changed.

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
- the destructive resize module and resize-triggered global gravity path no longer exist;
- formatting, strict Clippy, and the full all-features suite pass.

## Remaining SEDIMENT-001 authority

The following contracts remain open:

- bounded, retry-safe detached recovery — SEDIMENT-001C / issue #6;
- explicit immutable snapshot kinds and provenance — SEDIMENT-001D / issue #18.

Until those units are certified, detached catch-up and historical snapshots retain their current implementation limits and must not be described as fully conserved sediment authority.
