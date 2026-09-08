---
id: RAIN-005B2
kind: work
state: candidate
created: 2026-09-07
updated: 2026-09-07
authority: working
summary: Return runtime morphology to the owner-selected RAIN-004/A3 reference and suppress short-horizon focused-ingress bursts by replacing the IID golden-small branch coin with a seed-phased low-discrepancy schedule that preserves the same marginal bias and focused kernel.
---

# RAIN-005B2 — Golden low-discrepancy ingress

## Decision context

RAIN-005B1 and its R1/R2/R3 repairs all improved lateral centroid motion and width normalization but failed the unchanged thickness-CV Pareto floor. D1 then ruled out derived terrain/edge steering as the common CV-loss cause. D2 isolated the remaining mobility amplitude and found a real tradeoff rather than a missing derived point:

- full-footprint `J=L`: subset median `delta_cv=-0.004258796`, `delta_shift=+0.010949080`, `delta_span=+0.029795141`, with much better width normalization;
- `J=E*L`: subset median `delta_cv=+0.003926599`, but `delta_shift=-0.002215857`, `delta_span=-0.007911699`, and shift-vs-log-width `-0.592141840`;
- `J=E*W+`: subset median `delta_cv=+0.002771461`, but `delta_shift=-0.001741429`, `delta_span=-0.005749136`, and shift-vs-log-width `-0.606649138`.

D2 classified the search as `NO_DERIVED_SUBSET_FRONTIER`. RAIN-005B1 is therefore exhausted; no interpolation coefficient or fourth mobility law is authorized. B2 branches from the exact RAIN-005A3 runtime, which is RAIN-004 morphology, rather than inheriting rejected B1 runtime behavior.

## B2 target

The owner-selected RAIN-004 morphology remains the reference. B2 addresses a different concern: the broad focused component should not become perceptually nozzle-like when several grains are simultaneously airborne.

RAIN-005A3 showed that nozzle information is almost perfectly controlled by airborne population. B2 therefore targets *short-horizon concentration of focused authority*, not the marginal spatial kernel.

## Candidate semantics

### 1. Preserve RAIN-004 morphology authority

Keep unchanged:

- full active-width focus domain;
- RAIN-003/004 correlated meander and category rephase;
- golden-small marginal focus share `p = 1 / phi^2`;
- broad two-candidate nearer-to-focus sampler;
- free-site-only top ingress and FIFO pending mass;
- frozen `momentum-grounded-contact` settlement;
- PERF-001 optimized/reference semantics;
- appearance, persistence, catch-up and Advance authority.

### 2. Replace only IID focus-authority timing

RAIN-004 independently chooses focused versus uniform ingress with a Bernoulli draw of probability `p`. Even with a broad focused kernel, IID timing can randomly place many focused-authority ingresses inside one short visible-airborne interval.

B2 replaces only that Bernoulli timing with a seed-phased additive rotation over a 53-bit phase domain. The step is the nearest 53-bit representation of the same `p`; 53 bits are used because the existing probability sampler itself uses 53 random bits to match `f64` mantissa precision.

For every eligible ingress with more than one free site:

```text
phase += step(p)
focused = phase wrapped across 1
```

The initial phase is derived from the existing rain seed/state without consuming a new RNG draw. Each runtime decision still consumes one rain-RNG draw so B2 does not gratuitously remove RAIN-004's branch-decision RNG consumption.

### 3. No airborne-count runtime knob

The low-discrepancy sequence has the stronger property that for *any* consecutive window of `N` eligible ingress decisions:

```text
focused_count(N) is floor(N*p) or ceil(N*p)
```

Therefore the focused-authority count differs from `N*p` by less than one for every prefix and every contiguous window. `N_airborne` is the perceptual interpretation of that invariant, not a runtime parameter. B2 does not introduce an airborne threshold, window size, cooldown or rate limiter.

RAIN-005A3 measured expected airborne populations near 4 grains at p50 and 20 grains at p95. Test-only controls use those two evidence points to compare peak focused-authority load against the original IID scheduler; they are not runtime constants.

### 4. Marginal focused kernel stays unchanged

When a focused slot occurs, the target is still chosen by the exact RAIN-004 two-uniform-candidate nearer-to-focus tournament over currently free visible-top sites. Uniform slots remain uniform. B2 changes serial ordering only; it does not narrow, widen, steer, compensate or relocate the rain kernel.

## Machine gates

Focused gates must prove:

1. phase-quantized effective `p` differs from `1/phi^2` by at most one 53-bit phase quantum;
2. prefix and every tested contiguous-window focused-count discrepancy stay strictly below one;
3. no short period appears over bounded practical lags;
4. at A3 airborne proxy windows `N=4` and `N=20`, runtime peak focused count is strictly below deterministic RAIN-004 IID control for the same seed;
5. the 24k full-width target histogram remains broad and non-nozzle-like;
6. RAIN-004 full-width focus, meander/rephase, free-site FIFO, physics and PERF exactness remain green;
7. the exact A3 morphology fixture is byte-identical and a bounded IID control subset reproduces it at nine printed decimals.

The intentionally long paired morphology ensemble must rerun the exact 96 geometries x two seeds against the RAIN-004 fixture. Do not relax the existing Pareto directions:

```text
median delta CV    >= 0
median delta corr  <= 0
median delta TV    >= 0
median delta shift >= 0
median delta span  >= 0
```

If the scheduler passes perceptual/nozzle gates but fails morphology Pareto, B2 is blocked; do not tune a discrepancy coefficient locally.

## Human gate

Only after native machine PASS:

```text
testingcheats clear
testingcheats model hybrid
testingcheats fallspeed 1x
```

First judge the ordinary visible rain at 1x, specifically whether the focused component remains visually broad rather than forming transient vertical plumes. Then use `128x` for several strata/hours and compare morphology against remembered RAIN-004 behavior.

Desired result:

- falling dots look less bursty around the invisible focus;
- no regular temporal pulse is perceptible;
- no left/right Pepsi alternation is introduced;
- morphology is at least RAIN-004 on the existing Pareto metrics;
- settlement/avalanche character remains `momentum-grounded-contact`.

No human review is requested before machine PASS.
