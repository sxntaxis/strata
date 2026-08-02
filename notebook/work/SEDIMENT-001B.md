---
id: SEDIMENT-001B
kind: work
state: active
authority: working
created: 2026-08-02
updated: 2026-08-02
---

# SEDIMENT-001B — logical canvas and viewport projection

## Issue

Issue #7: terminal resize currently mutates canonical sediment through cropping, band repacking, overflow insertion, silent capacity loss, and a global gravity pass.

## Selected model

The persisted dot grid is the canonical logical canvas. Terminal dimensions are viewport dimensions only.

- A viewport resize changes `cell_width` and `cell_height` only.
- Canonical grid dimensions, grain coordinates, pending grains, physics metadata, and category mass remain unchanged.
- Rendering projects a centered horizontal, bottom-aligned window over the canonical canvas.
- A smaller viewport crops presentation without deleting hidden grains.
- A larger viewport pads presentation without stretching or relocating canonical grains.
- Restore installs the persisted canonical canvas directly instead of adapting it to the current terminal.
- No resize path invokes gravity, repacking, or ingress mutation.

The canonical canvas for a new empty profile is seeded once from the initial drawable viewport and then persists independently of later terminal geometry. Future explicit canvas migration or zoom policy is outside this unit.

## Acceptance proofs

- shrink → expand preserves the exact `SandState`;
- repeated oscillating resizes leave logical state byte-for-byte equivalent;
- total and per-category counts are unchanged even when viewport capacity is smaller than logical mass;
- coordinates, vertical order, and local category neighborhoods remain exact;
- hidden edge and upper grains reappear when the viewport expands;
- resize does not alter frame count, sweep direction, RNG state, pending order, or apply gravity;
- restore on a different viewport preserves the stored canonical dimensions and coordinates;
- render dimensions always equal the current terminal-cell viewport.

## Boundaries

- no zoom, compression, minimap, pan control, or canonical canvas migration;
- no detached recovery redesign (#6);
- no snapshot-kind redesign (#18);
- no change to the one-grain-per-second quantum;
- no change to sediment physics outside removal of resize side effects.
