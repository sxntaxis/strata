---
id: SEDIMENT-001B
kind: work
state: completed
authority: accepted
created: 2026-08-02
updated: 2026-08-02
---

# SEDIMENT-001B — logical canvas and viewport projection

## Issue

Issue #7: terminal resize destructively coupled display dimensions to sediment storage through cropping, band repacking, overflow insertion, capacity loss, and a global gravity pass.

## Accepted model

The persisted dot grid is the canonical logical canvas. Terminal dimensions are viewport dimensions only.

- A viewport resize changes `cell_width` and `cell_height` only.
- Canonical grid dimensions, grain coordinates, pending grains, physics metadata, and category mass remain unchanged.
- Rendering projects a centered horizontal, bottom-aligned window over the canonical canvas.
- A smaller viewport crops presentation without deleting hidden grains.
- A larger viewport pads presentation without stretching or relocating canonical grains.
- Restore installs the persisted canonical canvas directly instead of adapting it to the current terminal.
- No resize path invokes gravity, repacking, ingress placement, or other logical mutation.
- The obsolete resize module and edge-band policy are deleted.

The canonical canvas for a new empty profile is seeded once from the initial drawable viewport and then persists independently of later terminal geometry. Future explicit canvas migration or zoom policy is outside this unit.

## Certified proofs

- shrink → expand preserves the exact `SandState`;
- repeated oscillating resizes leave logical state exactly equivalent;
- total and per-category counts are unchanged even when viewport capacity is smaller than logical mass;
- coordinates, vertical order, local category neighborhoods, pending order, frame count, sweep direction, and RNG state remain exact;
- hidden edge and upper grains reappear when the viewport expands;
- restore on a different viewport preserves stored canonical dimensions and coordinates;
- render dimensions always equal the current terminal-cell viewport;
- all previous ingress, persistence, report, temporal, CLI, and TUI tests remain green;
- formatting and strict Clippy pass with all targets and features.

## Durable authority

- `docs/SEDIMENT_AUTHORITY.md` records the canonical canvas and projection contract;
- `docs/ARCHITECTURE.md` assigns topology to the logical grid rather than terminal geometry;
- STRATA-D025 forbids resize-driven canonical mutation;
- `notebook/work/SEDIMENT-001.md` advances to bounded detached recovery.

## Boundaries

- no zoom, compression, minimap, pan control, or canonical canvas migration;
- no detached recovery redesign (#6);
- no snapshot-kind redesign (#18);
- no change to the one-grain-per-second quantum;
- no change to sediment physics outside removal of resize side effects.
