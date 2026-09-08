---
id: RAIN-005B2D2
kind: work
state: complete
created: 2026-09-07
updated: 2026-09-08
authority: working
summary: Native factorial passed and selected the minimum-change frontier: original golden-small B2 authority plus a positional focus avulsion only at actually-entered stratum boundaries.
---

# RAIN-005B2D2 — Boundary-avulsion factorial

## Why this diagnostic exists

RAIN-005B2D1 falsified focus-authority probability as a one-dimensional fix. Raising `p_focus` strongly improves thickness CV, but adjacent-stratum correlation becomes progressively positive. At the smallest dyadic increase (`p=0.420593135547`), median CV is already `+0.016667215` and median shift is `+0.000658997`, while median correlation is `+0.074632316` and median span remains `-0.002880155`.

This suggests two different mechanisms are currently coupled:

1. **within-stratum concentration** — stronger focused authority increases relief/heterogeneity;
2. **between-stratum inheritance** — the same slowly moving focus remains spatially similar across successive category layers, increasing correlation.

D2 tests that interpretation directly instead of tuning another probability or meander speed.

## Frozen production semantics

Production remains exact RAIN-005B2:

- `p_focus = 1/phi^2`;
- B2 seed-phased low-discrepancy focused/uniform schedule;
- RAIN-004 full-width correlated meander;
- broad two-candidate focused kernel;
- free-site-only FIFO ingress;
- existing category-boundary heading rephase;
- weak broad terrain and soft edge steering;
- `momentum-grounded-contact` Classic settlement;
- PERF, persistence, renderer, scheduler, fallspeed, Advance, and Catch Up unchanged.

No production category-boundary teleport is authored in D2.

## Test-only boundary avulsion

When the probe sees a category change **at the moment the FIFO grain actually enters**, it may relocate the invisible focus to a new active-width locus before the existing RAIN-004 heading rephase.

The relocation introduces no distance scale:

- current focus must already exist;
- new focus is chosen pseudorandomly from every other active-width x position;
- same-x relocation is excluded;
- there is no jump length, corridor fraction, edge preference, terrain target, width coefficient, or viewport fraction;
- the probe uses counter-derived entropy without consuming the production rain RNG stream, so the factorial isolates positional boundary authority rather than downstream RNG-stream drift.

Physical grains never teleport. Only where subsequent focused rain is biased changes.

## Why category boundaries are allowed in this diagnostic

RAIN-004/B2 already treats an actually-entered category transition as a depositional rephase: it immediately arms a new heading decision and temporarily reduces directional inertia. D2 asks whether that existing semantic boundary should also be a positional decorrelation event. It does not respond to queued-but-blocked categories and does not use material identity beyond detecting a real transition.

## Three-arm bounded factorial

Use the first 64 exact A3 geometries and seed-slot 1:

1. `runtime-golden-small` — exact B2 control, `p=1/phi^2`, no positional boundary avulsion;
2. `boundary-avulsion-golden-small` — same `p`, boundary avulsion enabled;
3. `boundary-avulsion-toward-one-1of16` — smallest D1 authority increase, boundary avulsion enabled.

Total:

```text
64 geometries × 1 exact seed × 3 variants = 192 physical morphology runs
```

The runtime arm must reproduce the existing 64-row B2 control fixture at nine printed decimals before D2 is interpretable.

## Selection rule

For each boundary-avulsion arm, against the exact same RAIN-004/A3 subset:

```text
median delta CV    >= 0
median delta corr  <= 0
median delta TV    >= 0
median delta shift > 0
median delta span  > 0
shift_vs_log_width > exact 64-row RAIN-004 subset baseline
```

B2 anti-nozzle discrepancy semantics remain unchanged for the corresponding `p`; the diagnostic reports the implied `N=4` and `N=20` focused ceilings.

If both boundary arms pass, select only the smaller semantic change:

```text
boundary-avulsion-golden-small
```

If only the raised-authority arm passes, it becomes the sole candidate for a later full 192-run runtime repair. If neither passes, classify `NO_BOUNDARY_AVULSION_SUBSET_FRONTIER`; do not tune jump distance, probability, or category cadence locally.


## Native result — 2026-09-08

The exact D2 handoff passed transport, scope, fixture, compile, focused, B2-preservation, sample-level control-reproduction, formatting, strict Clippy, full-test, help-smoke, and PERF-001 gates. The 192-run factorial completed in 1803.24 seconds.

The two boundary-avulsion arms both passed the predefined subset Pareto floor. The minimum-change arm was selected exactly as frozen before execution:

```text
boundary-avulsion-golden-small
p = 0.381966011250
median delta CV    = +0.001302657
median delta corr  = -0.315065940
median delta TV    = +0.035007331
median delta shift = +0.046770031
median delta span  = +0.082278422
shift_vs_log_width = -0.026201417
subset_pass        = true
```

The raised-authority arm also passed, but is rejected by the predeclared smallest-semantic-change rule. D2 therefore establishes a bounded mechanism: retain B2's original `1/phi^2` authority and low-discrepancy anti-nozzle schedule; add only positional decorrelation at a category transition that has actually entered the physical FIFO. The next unit is RAIN-005B2R1, which promotes exactly that arm to runtime and requires the full 192-run A3 Pareto ensemble before any human morphology review.
