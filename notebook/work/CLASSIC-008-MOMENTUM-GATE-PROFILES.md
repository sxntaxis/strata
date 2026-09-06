---
id: CLASSIC-008
kind: work
state: candidate
created: 2026-09-06
updated: 2026-09-06
authority: working
summary: Compare three bounded alternatives to CLASSIC-007's fixed momentum drop-depth band: repose-relative eligibility, local tangent continuity, and a soft repose-relative edge, while preserving anchored as the default and momentum as the exact control.
---

# CLASSIC-008 — Momentum gate profiles

## Owner evidence

CLASSIC-007 is machine-green and human-improved. Its deep-cliff suppression fixed the worst early-fill artifact, but the owner wants to further perfect the transition into the visually successful mature-slope momentum behavior.

The owner explicitly requested the three previously proposed alternatives as selectable experiment profiles rather than replacing the accepted CLASSIC-007 control.

## Comparison profiles

The existing profiles remain unchanged, including:

```text
testingcheats classic experiment anchored
testingcheats classic experiment momentum
```

Add three profiles, all based on the same `anchored` mechanisms and the same maximum-one-bonus-hop transport boundary:

```text
testingcheats classic experiment momentum-repose
testingcheats classic experiment momentum-tangent
testingcheats classic experiment momentum-soft
```

`anchored` remains the default. `momentum` remains the exact CLASSIC-007 fixed `drop_depth = 2..=3` control. Selection stays non-retroactive; `testingcheats fillhalf` provides the clean human comparison fixture.

### A — `momentum-repose`

Reuse the local repose already governing the prospective bonus source column. Let `R` be that column's current local repose and `D` the prospective bonus receiving-column drop depth.

```text
bonus eligible iff D in [R, R + 1]
```

This removes the global 2..=3 magic band and asks whether the continuation is close to the material's own local stability threshold. It adds no new RNG consumption.

### B — `momentum-tangent`

Use bounded local geometry only. The prospective receiving column must be near the moving grain (`D` in `1..=3`), then sample exactly one additional column in the same direction from the next row. Let that forward drop be `F`.

```text
D in 1..=3
F in 1..=3
abs(D - F) <= 1
```

This approximates a locally continuous surface tangent without angles, global slope calculation, or another transport hop. The forward sample is eligibility-only.

### C — `momentum-soft`

Use local repose as in A but soften the upper edge. Let `R` be local repose and `D` the prospective drop.

```text
D == R       -> bonus eligible
D == R + 1   -> 60% eligible
otherwise    -> no bonus
```

Only the `R + 1` case consumes one physics RNG draw. This profile deliberately tests whether a probabilistic transition removes the visual on/off boundary without opening cliffs.

## Hard boundaries

All four momentum comparison profiles (`momentum` plus A/B/C) retain:

- anchored texture/memory/slope-bias/rare-anchor base semantics;
- ordinary Classic diagonal law unchanged;
- at most one same-direction bonus diagonal;
- no persistent velocity, rolling state, multi-hop loops, trajectory state, front machinery, global slope, or angle solver;
- exact mass conservation;
- unchanged `fill` / `fillhalf`;
- unchanged Oslo, renderer, persistence, schema, provenance, category contracts, and production H4 semantics.

## Human comparison

Use a fresh centered half fill after every selection:

```text
testingcheats classic experiment anchored
testingcheats fillhalf

testingcheats classic experiment momentum
testingcheats fillhalf

testingcheats classic experiment momentum-repose
testingcheats fillhalf

testingcheats classic experiment momentum-tangent
testingcheats fillhalf

testingcheats classic experiment momentum-soft
testingcheats fillhalf
```

Judge two phases separately: early near-vertical fill edges and mature diagonal avalanche slopes. The desired winner minimizes lateral-looking early hops while preserving or improving the entertaining coherent continuation on mature relief. Do not promote any momentum profile to default until the owner chooses after this comparison.
