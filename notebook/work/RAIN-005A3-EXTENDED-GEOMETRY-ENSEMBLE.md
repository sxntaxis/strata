---
id: RAIN-005A3
kind: work
state: candidate
created: 2026-09-07
updated: 2026-09-07
authority: working
summary: Extend the RAIN-004 statistical reference from the small/medium A2 geometry domain into a logarithmic continuous terminal-area/aspect domain with paired seeds, explicit geometry-vs-seed dependence measurement, and a dense analytical nozzle sweep; no runtime behavior changes.
---

# RAIN-005A3 — Extended geometry ensemble

## Why A3 exists

RAIN-005A2 successfully established that RAIN-004 has useful morphology/nozzle diagnostics, but its 16 native morphology cases occupied a relatively small terminal domain. That was enough to detect run-to-run variation and reject optimization against one owner screenshot, but not enough to claim viewport independence for fullscreen, very large terminals, heavily tiled panes, portrait terminals, ultrawide panes, or unusual font zooms.

The owner explicitly allows a long native machine pass. RAIN-005A3 therefore spends machine time instead of human review.

RAIN-004 remains the **best-so-far reference family**, not a perfection/freeze target. RAIN-005A3 changes no runtime morphology.

## Continuous geometry domain

Do not enumerate popular monitor resolutions.

The morphology probe samples terminal geometry in two continuous logarithmic coordinates:

- terminal area;
- aspect ratio.

The low-discrepancy Halton sequence then maps those coordinates into terminal widths/heights bounded only for test-resource safety:

```text
terminal width:  20 .. 420 cells
terminal height: 10 .. 140 cells
sampled target area: 300 .. 50,000 terminal cells
sampled target aspect: 0.18 .. 18.0
```

Those are **test-domain coverage bounds**, never runtime morphology constants or canonical user resolutions.

The test asserts that the realized set retains narrow-pane, large-width, shallow, tall, portrait, ultrawide and large-area coverage so a future refactor cannot silently collapse the ensemble back to conventional desktop sizes.

## Paired seeds: separate geometry from stochastic luck

A2 used one deterministic seed per geometry, which confounded geometry effects with stochastic run variance.

A3 uses:

```text
96 geometries x 2 independent deterministic seeds = 192 morphology runs
```

Each run remains normalized to one analytically derived one-dot-excess mound timescale per category and five category phases.

For every run report the existing independent morphology primitives:

- mean thickness CV;
- mean adjacent profile correlation;
- mean adjacent profile total variation;
- mean adjacent centroid shift;
- total centroid span;
- mound scale in equivalent layers.

Across all 192 runs report p05/p10/p25/p50/p75/p90/p95 plus min/max.

For each paired geometry report seed-to-seed absolute deltas. This lets later design distinguish:

```text
"this viewport shape changes morphology"
```

from:

```text
"RAIN-004 is stochastic and these two runs simply differed"
```

## Geometry dependence

Average the two seeds for each geometry, then report Pearson dependence of each morphology primitive against:

- log terminal width;
- log terminal height;
- log terminal area;
- signed log aspect ratio;
- absolute log aspect extremeness.

These are measurements, not pass/fail thresholds. The purpose is to discover whether RAIN-004's apparent scale stability survives much larger geometry and whether a future normalized RAIN-005B law needs to account for any remaining systematic dependence.

## Dense analytical nozzle ensemble

The nozzle calculation is cheap enough to sample much more densely than physical morphology.

A3 therefore uses 4096 analytical geometries over:

```text
dot width:  40 .. 840
dot height: 40 .. 560
focus position: full normalized width
open-sky fraction: 0.02 .. 0.98
```

Width and height are logarithmically sampled; focus and sky fraction use independent low-discrepancy coordinates.

Report distributions for:

- expected canonical-1x airborne population;
- KL bits per grain;
- total visible nozzle information;
- normalized kernel RMS width;
- mound scale in equivalent layers.

Also report dependence/correlation including:

- nozzle information vs expected airborne population;
- nozzle information vs width/height/sky fraction;
- KL vs width;
- KL vs distance from the nearest edge;
- kernel width vs edge distance;
- mound-equivalent-layer scale vs width.

This directly tests the owner's hypothesis that simultaneous visible airborne population is the natural perceptual variable behind nozzleness.

## Hard boundary

RAIN-005A3 is test-only evidence.

It must not change:

- RAIN-004 bias/kernel;
- correlated meander;
- category rephase;
- terrain or edge steering;
- free-site ingress;
- Classic settlement or RNG ordering in runtime;
- PERF-001 runtime behavior;
- renderer/appearance;
- persistence/schema;
- Advance/catch-up.

No A3 metric becomes runtime authority or an acceptance threshold.

## Runtime expectation

The extended morphology probe is intentionally long-running. One hour or more is acceptable on the owner's native machine. Validation must not reduce case count or geometry range merely to shorten execution time.

## Next decision

Only after A3 is native-green and its raw outputs are available should RAIN-005B be authored.

RAIN-005B should use normalized/equivalent-layer/kernel relationships and must be compared against the **extended RAIN-004 family**, with the design goal of equal-or-greater heterogeneity and no airborne-nozzle regression. RAIN-004's current magic constants remain replaceable.
