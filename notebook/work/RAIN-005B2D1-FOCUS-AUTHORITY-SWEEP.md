---
id: RAIN-005B2D1
kind: work
state: candidate
created: 2026-09-07
updated: 2026-09-07
authority: working
summary: Test whether B2's low-discrepancy anti-nozzle scheduler can recover the tiny shift/span Pareto losses by increasing only focused-authority probability through a derived dyadic bracket toward the physical p=1 limit; production runtime remains exact B2 during this diagnostic.
---

# RAIN-005B2D1 — Focus-authority sweep

## Why this diagnostic exists

RAIN-005B2 successfully suppressed short focused-authority bursts without changing the RAIN-004 focused kernel or meander:

- `N=4`: B2 maximum focused slots `2`, deterministic RAIN-004 IID control maximum `4`;
- `N=20`: B2 maximum focused slots `8`, deterministic RAIN-004 IID control maximum `18`.

Its full 192-run morphology result was near-neutral and failed only the two lateral medians:

- median `delta_cv=+0.000731718`;
- median `delta_corr=-0.009411262`;
- median `delta_tv=+0.001898856`;
- median `delta_shift=-0.000010301`;
- median `delta_span=-0.003061554`.

This is qualitatively different from the B1 family: B2 does not pay for lateral gains with a CV loss. The owner therefore explicitly questioned whether RAIN-004's `1/phi^2` (~38.2%) focus share is still being treated as an unjustified magic number and whether focus authority itself may be the variable that can improve CV and shift/span together.

## Diagnostic question

Hold fixed:

- B2 low-discrepancy serial scheduler;
- RAIN-004 full-width correlated focus meander/rephase;
- broad two-candidate focused kernel;
- free-site-only ingress/FIFO;
- frozen `momentum-grounded-contact` settlement;
- per-stratum mass / A3 mound-time measurement;
- physics, PERF, persistence, renderer, catch-up and Advance semantics.

Vary only the marginal probability that an eligible ingress receives focused authority.

## Why the sweep is dyadic rather than hand-tuned

The current value is

```text
p0 = 1 / phi^2
```

The physical upper bound is

```text
p = 1
```

D1 does not invent candidate percentages. It brackets the interval from the current value toward the upper bound by repeated powers of two:

```text
runtime:       p0
toward-one/16: p0 + (1-p0)/16
toward-one/8:  p0 + (1-p0)/8
toward-one/4:  p0 + (1-p0)/4
toward-one/2:  p0 + (1-p0)/2
```

These are test-only search coordinates, not runtime constants and not a claim that any dyadic level is physically privileged.

## Low-discrepancy invariant at every tested p

For every tested probability, the B2 scheduler still uses the 53-bit additive phase rotation. The effective probability is the nearest representable phase step and must differ from the requested value by no more than one phase quantum.

For every tested contiguous eligible window of length `N`, focused count must remain within less than one slot of `N*p`. Thus the anti-burst mechanism itself is unchanged; increasing `p` increases the expected focused share but does not reintroduce IID clustering.

The diagnostic reports the resulting maximum focused-slot count implied by that invariant for the two A3 airborne evidence windows `N=4` and `N=20`.

## Ensemble

Use the first 64 exact A3 geometries and seed-slot 1, matching the established D1/D2 bounded factorial convention:

```text
64 geometries × 1 exact seed × 5 authority levels = 320 physical morphology runs
```

The exact RAIN-005A3 fixture remains the reference. A new 64-row control fixture is extracted directly from the already-native B2 full-ensemble output. The runtime-B2 arm must reproduce those emitted B2 samples at nine printed decimals before any sweep interpretation is accepted.

## Subset frontier rule

For each authority level report median deltas against RAIN-004/A3:

```text
CV    >= 0
corr  <= 0
TV    >= 0
shift > 0
span  > 0
```

Also require candidate `shift_vs_log_width` to be strictly better (less negative / greater) than the exact same 64-row RAIN-004 subset baseline.

If multiple non-runtime levels pass, select exactly one:

```text
smallest authority increase above p0 that passes every subset gate
```

This is a diagnostic selection only. It does not mutate production runtime and does not authorize human review. The selected level, if any, becomes the sole candidate for a later full 192-run B2 repair pass.

If no level passes, classify `NO_DYADIC_SUBSET_FRONTIER`; do not interpolate or tune another percentage locally.

## Native outcome

Native validation completed the full 320-run test-only sweep with exact B2 control reproduction, all focused discrepancy invariants, fmt/clippy/full tests/help/PERF preservation green.

No tested authority level formed a valid subset frontier:

- runtime `p=0.381966011250`: `delta_cv=+0.001049189`, `delta_corr=-0.011020778`, `delta_tv=+0.001974393`, `delta_shift=-0.002472747`, `delta_span=-0.006201151`;
- `p0+(1-p0)/16 = 0.420593135547`: `delta_cv=+0.016667215`, `delta_corr=+0.074632316`, `delta_tv=+0.000629622`, `delta_shift=+0.000658997`, `delta_span=-0.002880155`;
- `p0+(1-p0)/8 = 0.459220259844`: `delta_cv=+0.028029527`, `delta_corr=+0.120250413`, `delta_tv=-0.000300172`, `delta_shift=-0.001093282`, `delta_span=+0.000656831`;
- `p0+(1-p0)/4 = 0.536474508438`: `delta_cv=+0.060831885`, `delta_corr=+0.248841722`, `delta_tv=+0.000601510`, `delta_shift=-0.000609977`, `delta_span=+0.000850420`;
- `p0+(1-p0)/2 = 0.690983005625`: `delta_cv=+0.127155625`, `delta_corr=+0.393787863`, `delta_tv=+0.000350142`, `delta_shift=+0.000338781`, `delta_span=+0.004581047`.

Classification: `NO_DYADIC_SUBSET_FRONTIER`.

The causal interpretation is sharper than merely "authority does not work": increasing focused authority strongly increases within-stratum thickness CV, and at the smallest increase already turns median shift positive, but it also makes adjacent strata increasingly positively correlated. The failure therefore points to excessive cross-stratum inheritance of the same focus locus, not insufficient within-stratum focus strength. Do not interpolate another probability. RAIN-005B2D2 tests whether a focus relocation exactly at an actually-entered stratum boundary can separate those two effects.
