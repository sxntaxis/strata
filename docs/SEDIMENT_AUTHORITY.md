# Sediment authority

Status: partially implemented and certified
Program: SEDIMENT-001
Current completed unit: SEDIMENT-001A
Issues completed by this unit: #16, #26
Last reviewed: 2026-08-02

## Purpose

Sediment is accountable visual history derived from elapsed time. Chronological sessions remain the exact time authority, while sediment preserves its own explicit mass, category, topology, and projection obligations.

The full SEDIMENT-001 program covers ingress, viewport projection, detached recovery, and historical snapshot identity. This document records accepted sediment authority as each bounded unit becomes certified.

## Logical mass

A due grain exists exactly once as either:

- a placed physical grain in the logical dot grid; or
- a pending grain waiting for an ingress position.

Physical blockage is not permission to discard elapsed-time mass. `grain_count` therefore represents placed plus pending logical mass. Pending grains retain category identity, preserve FIFO order, and are persisted with `SandState`.

A newly due grain enters the pending reservoir first. The engine chooses a randomized starting column and scans the entire ingress row exactly once. It remains pending only when every ingress column is occupied. Normal simulation updates and resize completion retry pending ingress without creating another grain.

Clearing all sediment clears both placed and pending mass. Category-specific clearing and removal apply to both forms. Unknown category identities encountered during restore follow the existing explicit normalization to idle rather than disappearing.

## Dimension units

Terminal geometry and Braille-dot geometry are distinct units:

- `cell_width` and `cell_height` are drawable terminal-cell dimensions;
- `grid_width_dots` and `grid_height_dots` are physical Braille-dot grid dimensions;
- one terminal cell projects `dot_width × dot_height` logical dots.

Rendering iterates terminal cells and emits exactly one Braille character per drawable cell. Simulation, persistence, snapshots, and capacity calculations use dot-grid dimensions. Callers must not compare, divide, or infer these units through ambiguous `width` or `height` fields.

## Persistence compatibility

`SandState.pending_grains` is backward-compatible:

- older JSON without the field loads as an empty pending reservoir;
- an empty reservoir is omitted during serialization, preserving prior ordinary state shape;
- nonempty pending mass is serialized in category-preserving order;
- storage, SQLite state, detached checkpoints, catch-up projections, and report previews use the same state contract.

## Certification for SEDIMENT-001A

The unit proves:

- a grain finds the only free ingress column regardless of randomized starting position;
- a fully blocked grain remains logical and later enters when ingress reopens;
- logical and category mass survive snapshot/restore with pending state;
- terminal output dimensions equal the requested cell dimensions exactly;
- named cell and dot-grid dimensions propagate through TUI rendering, catch-up projection, report previews, persistence, and fault tests;
- formatting, strict Clippy, and the full all-features suite pass.

## Remaining SEDIMENT-001 authority

SEDIMENT-001A does not establish the following contracts:

- viewport-independent canonical topology and non-destructive resize — SEDIMENT-001B / issue #7;
- bounded, retry-safe detached recovery — SEDIMENT-001C / issue #6;
- explicit immutable snapshot kinds and provenance — SEDIMENT-001D / issue #18.

Until those units are certified, viewport resize, catch-up, and historical snapshots retain their current implementation limits and must not be described as fully conserved sediment authority.
