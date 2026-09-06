---
id: SEDIMENT-015H
kind: work
state: candidate
created: 2026-09-05
updated: 2026-09-05
authority: working
summary: Preserve the owner-preferred SEDIMENT-015G-R2 physics, custody, support lanes, chronology and truthful raster while making only large downward observed-edge segments take visibly longer and use a gravity-shaped Y interpolation.
---

# SEDIMENT-015H — Distance-aware drop playback

## Entry evidence

SEDIMENT-015G-R2 is the strongest owner-observed visual baseline. SEDIMENT-016 was rejected as a giant regression and is not part of this lineage.

D4 live provenance then observed the exact owner rainbow run rather than another synthetic trigger. The evidence separated physics from presentation:

```text
green:  movement_entries=3    hops=64
 yellow: movement_entries=585  hops=20437
 red:    movement_entries=1283 hops=52150
 purple: movement_entries=2083 hops=88736
 blue:   movement_entries=3322 hops=180104
 cyan:   movement_entries=4317 hops=248180 max_hops=148 max_abs_dx=148
```

The upper-layer mobility bias is therefore real frozen Oslo-front behavior, not renderer fabrication. D4 also observed 1401 cyan units below the original green-band height and traced adjacent physical cyan paths into that region.

The remaining owner-visible defect is narrower: the first cyan failures can look like teleportation. At the post-resize wall edge, a physically valid adjacent hop can cross only one lattice site horizontally while its observed Y drops by a very large number of Braille dots because the neighboring column is nearly empty. 015G currently gives that cliff-sized segment the same fixed `UNIT_SEGMENT_STEP = 0.24` cadence as an ordinary one-dot surface hop.

## Contract

015H changes presentation timing only.

Preserve exactly:

- SEDIMENT-008 frozen physics;
- SEDIMENT-015G-R2 final-batch visual-support lanes;
- SEDIMENT-015E coherent physical-quantum replay barrier;
- SEDIMENT-015D truthful bounded raster;
- SEDIMENT-015A one-unit MotionId/CategoryId custody and exact stack order;
- D4 debug-only live provenance;
- `testingcheats fill` Fixture-category provisioning;
- rain, resize, persistence and schema behavior.

Add a new presentation-only profile layered over 015G and selected by `testingcheats model oslo-vessel-front-grains`.

### Short segments

For downward observed Y change `<= 4` dots, preserve the exact 015G cadence and linear interpolation:

```text
progress_step = 0.24
vertical_fraction = progress
```

### Large downward segments

For a larger observed fall, derive only the visual duration from already-finalized `start_y` and `target_y`:

```text
drop = target_y - start_y
frames = min(18, 5 + ceil(0.9 * sqrt(drop)))
progress_step = 1 / frames
vertical_fraction = progress^2
```

This is not new sediment physics. It is a bounded presentation clock and gravity-shaped interpolation between two immutable endpoints that physics already produced.

Important constraints:

- no future route prediction;
- no new intermediate physical sites;
- no target change;
- no CategoryId change;
- no shadow/custody change;
- no motion coalescing;
- no replay longer than 18 visual frames for one observed edge;
- upward/small segments retain 015G behavior.

## Why this is bounded

The D4 cyan trace proves that cyan genuinely reaches low/outward regions. 015H must therefore not alter where cyan goes. It only makes a high-relief one-edge transition perceptually legible instead of compressing a cliff fall into roughly five frames.

The square-root duration scaling follows the qualitative time-vs-distance relationship of gravitational fall without claiming that terminal-grid Y is a physical length or that this animation integrates real gravity. The 18-frame cap prevents a presentation cliff from stalling the coherent replay indefinitely.

## Required native evidence

1. 015H constructor extends 015G with identical columns, thresholds and RNG states.
2. Short/small downward hops retain `UNIT_SEGMENT_STEP` and linear Y exactly.
3. Large downward hops take more than the baseline cadence but no more than 18 frames.
4. Large-drop Y interpolation is monotonic/gravity-shaped and ends exactly at the finalized 015G target.
5. A synthetic tall-wall adjacent hop has the same target in 015G and 015H; only 015H playback is slower.
6. Frozen-front parity, exact final stack, avalanche moves and zero unit misses remain exact.
7. All 015G-R2/D4, 015F→015A, conservative FlowViz and focused Oslo regressions remain green.
8. Truthful raster scaling remains unchanged; no PBD/full-height search returns.
9. fmt, strict Clippy, full tests, help smoke and command parser pass.

## Human gate

Only after native green, reproduce the rainbow wall with normal `testingcheats fill`; `64x` is not required.

Question:

> At the first wall-edge failures, does cyan now visibly fall through the large vertical relief rather than appearing to jump from the top band to the low flank, while the rest of the 015G-R2 collapse remains unchanged?
