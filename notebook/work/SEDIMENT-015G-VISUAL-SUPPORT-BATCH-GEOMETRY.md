---
id: SEDIMENT-015G
kind: work
state: active
created: 2026-09-05
updated: 2026-09-05
authority: working
summary: Replace 015F transient rolling-depth geometry with batch-final presentation lanes that are contiguous above the delayed visual shadow, while preserving 015E coherent chronology, 015D truthful O(N) raster, unit custody, and frozen Oslo physics.
---

# SEDIMENT-015G — Visual-support batch geometry

## Entry condition

SEDIMENT-015F improved direct per-grain Y continuity, but the owner rainbow-collapse gate still showed large diagonal black channels inside the apparent avalanche and detached rails on both flanks.

Human result:

```text
FAIL_HOLLOW_AVALANCHE_PRESENTATION
```

## Corrected source diagnosis

A second source inspection corrected an earlier diagnosis: the active 015F constructor inherits the micro carrier path, and truthful mode disables PBD relocation while still replaying `active.target_y`. The renderer was therefore already consuming the captured segment target.

The remaining defect is earlier, at target assignment.

015F assigns a rolling lane while authoritative grains are processed sequentially:

```text
target_y = physical settled height
         + rolling_depth_at(destination)
```

Two independent presentation errors follow.

### A. Hidden physical support

`columns[destination]` can contain physically settled mass whose visual carrier has not yet arrived/revealed. A later rolling grain is therefore placed above support that is not present in the delayed visual shadow.

```text
physical bed (future)      ███████
visual shadow (now)        ███
rolling target             •
visible result             •
                           <gap>
                           ███
```

### B. Transient sequential rolling depth

During `advance_rolling_grains`, a grain moving into site X can count rolling grains at X that have not yet been processed in the same authoritative quantum. Those grains may subsequently move away or settle. The incoming carrier retains the higher lane that was assigned from this transient intermediate queue state.

This manufactures hollow rails even though the final rolling population at X no longer occupies the lower lanes.

## 015G contract

015G preserves 015F as an explicit control and adds a visual-support profile selected by:

```text
testingcheats model oslo-vessel-front-grains
```

For direct visual-support transport:

1. recruitment/hop records source Y immediately;
2. target Y remains pending during authoritative mutation;
3. the complete Oslo quantum finishes;
4. the final surviving rolling population is known;
5. lanes are assigned deterministically in final rolling-queue order;
6. lane 0 is immediately above the current **visual shadow**, not hidden authoritative settled mass;
7. lane N is exactly N dots above lane 0;
8. the finalized target is immutable during replay.

Formula for a finalized lane:

```text
target_y = grid_height
         - visual_shadow_height(site)
         - final_rolling_depth(site)
         - 1
```

The visual shadow is the presentation support authority. `columns` remains the physics authority.

## Why batch finalization is required

The physical front solver intentionally processes a bounded deque sequentially for deterministic mutation. Presentation must not expose those transient implementation states as simultaneous geometry.

015G therefore observes the **post-quantum rolling population**, not intermediate deque occupancy.

This does not change:

- which grain moves;
- direction;
- relief;
- erosion/support recruitment;
- coast budget;
- thresholds/RNG;
- settlement/discharge;
- CategoryId custody.

Only presentation lane assignment is delayed until the authoritative quantum boundary already established by 015E.

## Preserved foundations

015G must preserve:

- 015F direct source-Y continuity as a control;
- 015E coherent one-quantum replay barrier;
- 015D truthful O(N)-class raster and live rain;
- 015A one-unit MotionId and exact ordered custody;
- no future-route prediction;
- exact total/category mass and final stack;
- SEDIMENT-008 frozen front physics;
- persistence/schema and canonical production profile.

## Required native evidence

1. 015G constructor extends 015F with identical columns, thresholds and RNG.
2. New rolling target remains pending until batch finalization.
3. Hidden authoritative settled mass cannot lift a carrier above absent visual support.
4. Multiple final rolling grains at one site occupy contiguous visual lanes with no internal hole.
5. Later visual-shadow mutation cannot bend an already-finalized segment.
6. 015F direct control regressions remain unchanged and green.
7. 015E/015D/015C/015B/015A and conservative regressions remain green.
8. Frozen-front parity, exact final stack, avalanche moves and zero unit misses remain exact.
9. Truthful raster/scaling remains O(N)-class and no PBD/full-height search returns.
10. fmt, strict Clippy, full tests, help smoke and command parser pass.

## Human question

Only after machine green:

> Do the large diagonal voids/hollow rails disappear when rolling carriers are packed contiguously above the delayed visual bed using only the final rolling occupancy of each coherent physical quantum?
## R1 native-gate correction

The first native run stopped at `unit_visual_support_final_batch_lanes_are_contiguous_above_visible_bed` before exercising 015G semantics. The synthetic fixture attempted to recruit `source_a -> destination` across two lattice sites, so `record_flowviz_mobile_entry_with_custody` correctly returned `None` under the existing adjacent-edge contract. R1 changes only the fixture: the two sources are now the immediate left and right neighbours of one destination, with matching Right/Left directions. Runtime source is unchanged. The contiguous-lane expectation remains exactly `base, base - 1, ...`.

## R2 — finalized playback is the rendered position

R1 passed the corrected contiguous-lane fixture, constructor control, and visible-support gate, then failed `unit_visual_support_playback_uses_finalized_target_after_shadow_changes`. The failure was semantic, not another fixture error.

The finalized `active.target_y` remained correct, but the 015G constructor inherits `flowviz_unit_micro = true`; `advance_unit_flowviz()` therefore applied `UNIT_MICRO_TRACK_BLEND` after computing the coherent segment interpolation. The carrier's rendered `x/y` lagged behind `ideal_x/ideal_y`, even though 015G had already finalized a visual-support lane. A later shadow mutation could therefore expose a hole between the delayed visual bed and the lagging carrier.

R2 changes only the 015G visual-support profile:

```text
finalized segment geometry
        ↓
coherent interpolation (ideal_x, ideal_y)
        ↓
rendered carrier x/y exactly
```

The inherited micro tracking blend remains unchanged for 015F and all earlier control profiles. 015G still uses the truthful raster, coherent physical-quantum barrier, live-rain policy, exact MotionId/CategoryId custody, and batch-final lane assignment. Oslo physics, thresholds, RNG, settlement/discharge authority, persistence and schema remain untouched.

R2 native validation must first close the exact immutable-playback failure, then re-run all 015G support semantics and earlier controls.
