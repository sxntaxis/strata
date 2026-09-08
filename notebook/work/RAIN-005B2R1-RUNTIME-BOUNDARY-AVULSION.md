---
id: RAIN-005B2R1
kind: work
state: candidate
created: 2026-09-08
updated: 2026-09-08
authority: working
summary: Promote D2's minimum-change frontier to Classic WanderingFocus runtime: keep B2 golden-small low-discrepancy ingress unchanged and relocate only the invisible focus at an actually-entered category boundary.
---

# RAIN-005B2R1 — Runtime boundary avulsion

## Selection basis

RAIN-005B2D2 isolated the cross-stratum inheritance mechanism and produced a clean subset frontier. At the original B2 authority (`p=1/phi^2`), adding only an actually-entered boundary avulsion changed the 64-geometry seed-1 medians versus RAIN-004 to:

```text
CV    +0.001302657
corr  -0.315065940
TV    +0.035007331
shift +0.046770031
span  +0.082278422
shift_vs_log_width -0.026201417
```

The higher-authority arm also passed, but the frozen selection rule required the smallest focus authority that passed all subset gates. R1 therefore retains the original golden-small authority exactly.

## Runtime semantics

For `ClassicRainMode::WanderingFocus` only:

1. A queued category change has no focus authority while blocked.
2. When a grain of a category different from the last actually-entered category can enter, the invisible focus is relocated before the existing category heading rephase.
3. The relocation chooses pseudorandomly from every other active-width x locus; same-x is excluded.
4. No jump-length scale, corridor fraction, viewport fraction, edge preference, terrain target, probability, cooldown, or additional timing constant is introduced.
5. Boundary entropy is counter-derived and does not consume the production rain RNG stream.
6. Physical grains do not teleport. Only where subsequent focused rain is biased changes.
7. Repeated grains of the same category do not trigger relocation or rephase.
8. `ClassicRainMode::Uniform` remains unchanged.
9. `clear()` resets boundary-event state along with the existing rain-focus state.

## Frozen B2 semantics

R1 does not change:

- `p_focus = 1/phi^2`;
- the seed-phased low-discrepancy focused/uniform scheduler;
- the focused two-candidate broad kernel;
- full active-width rain support;
- RAIN-004 correlated meander between boundaries;
- weak broad terrain steering and soft real-edge steering;
- free-site-only FIFO ingress;
- `momentum-grounded-contact` Classic settlement;
- PERF/fallspeed/renderer/scheduler/Advance/Catch Up behavior.

Historical B2/Rain-004 reconstruction remains test-only through explicit diagnostic overrides; production defaults to the R1 semantics.

## Machine gate

R1 must pass the full exact A3 paired ensemble:

```text
96 geometries x 2 exact seeds = 192 physical morphology runs
```

Against the frozen RAIN-004 fixture:

```text
median delta CV    >= 0
median delta corr  <= 0
median delta TV    >= 0
median delta shift > 0
median delta span  > 0
shift_vs_log_width > -0.412362
```

B2 anti-nozzle discrepancy invariants remain mandatory (`N=4` max focused slots `2`; `N=20` max `8`). Runtime category-boundary tests must prove actual-entry authority, same-category non-authority, full-width aperiodic exploration, and no rain-RNG stream drift.

The full probe must also reproduce the 64 seed-slot-1 samples from the D2-selected arm at the existing nine-decimal evidence precision before its full-ensemble medians are interpretable. That frozen promotion-control fixture is `tests/fixtures/rain_005b2d2_selected_boundary_control.csv`.

If the full machine gate passes, do not tune further. Advance exactly this candidate to a human morphology gate against RAIN-004. If the full gate fails, stop and analyze the geometry/seed distribution; local semantic tuning is forbidden.
