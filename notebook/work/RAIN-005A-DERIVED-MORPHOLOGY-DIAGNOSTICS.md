---
id: RAIN-005A
kind: work
state: candidate
created: 2026-09-07
updated: 2026-09-07
authority: working
summary: Measure RAIN-004's natural airborne, rain-kernel, equivalent-layer, mound-formation, and adjacent-strata scales without changing rain or Classic behavior, so a later derived morphology law can remove magic constants while targeting at least RAIN-004's heterogeneity.
---

# RAIN-005A — Derived morphology diagnostics

## Owner decision

RAIN-004 is the owner's **best-so-far morphology baseline** and becomes the reference/control for the next refinement. It is not declared perfect and its current golden-ratio bias or meander timing constants are not frozen as final semantics.

The next morphology design must target:

- equal or greater stratigraphic heterogeneity than RAIN-004;
- no regression to a visually detectable falling-rain nozzle;
- no return to RAIN-002's bland/over-compensated envelopes;
- no deterministic Pepsi-like left/right oscillation.

RAIN-005A is measurement-only. It must not retune rain, focus motion, category rephase, Classic settlement, performance, or rendering.

## Natural scales to measure

The canonical temporal anchor is existing Strata semantics:

```text
1 ingress grain / simulated second
```

From the actual viewport and current Classic geometry derive/report:

- active width/height in dot coordinates;
- exact currently visible airborne count, where an occupied grain with an air gap beneath its vertical column is airborne under the same grounded-contact meaning used by Classic;
- current grounded count;
- mean open vertical drop distance from free ingress sites;
- geometry-derived expected canonical-1x airborne population using the existing gravity cadence;
- bottom-connected surface mean/stddev/min/max relief;
- one equivalent layer (`EL`) as one grain per active dot column;
- equivalent layers per canonical simulated hour.

## Rain-kernel diagnostics

Do not bin the rain distribution. Analyze the existing RAIN-004 broad two-candidate focus tournament directly.

For both the current free-site set and an unobstructed full-width reference, report:

- focus-biased share used by RAIN-004 as a **reference input**, not accepted future authority;
- KL divergence from uniform, in bits per ingress;
- expected information visible in one airborne population (`N_airborne * KL`);
- RMS focus-kernel width in dots and as a width fraction.

Use the unobstructed reference kernel to derive a parameter-free excess-deposition scale:

- positive excess mass per ingress over uniform;
- effective positive-excess width by participation ratio;
- grains/time required to build one dot of excess relief over that effective width;
- mound scale expressed in equivalent layers.

These values are diagnostics. RAIN-005A does not introduce a nozzle threshold or use the metrics to steer runtime behavior.

## Stratigraphic heterogeneity diagnostics

For every category present in the **bottom-connected settled pile** of the active viewport, derive a thickness profile over x and report:

- mass;
- normalized lateral centroid;
- mean thickness;
- thickness standard deviation;
- thickness coefficient of variation;
- mean vertical coordinate.

Order categories diagnostically bottom-to-top by mean vertical coordinate. This is not historical provenance.

For adjacent profiles report independently, without collapsing them into a magic composite score:

- Pearson thickness-profile correlation where defined;
- normalized-profile total variation distance;
- normalized centroid shift.

Also report the arithmetic means of the defined pair metrics.

## Debug surface

Add a read-only debug command:

```text
testingcheats classic rainmetrics
```

It writes:

```text
$XDG_CACHE_HOME/strata/classic-rain-metrics.txt
```

(or the existing HOME cache fallback).

The report must explicitly state that RAIN-004 is the owner-selected best-so-far **reference**, not a perfection/freeze target, and that no reported metric is currently a runtime steering authority or acceptance threshold.

## Exact preservation gates

RAIN-005A must preserve RAIN-004 exactly:

- full-width correlated meander;
- golden-small broad focus bias;
- category-boundary rephase;
- free-site ingress;
- fallspeed debt ownership and immediate slowdown;
- PERF-001 exact optimized Classic behavior/capacity;
- `momentum-grounded-contact`;
- appearance/theme/color behavior;
- persistence/schema/catch-up/Advance authority.

The diagnostics call itself must be read-only with respect to grid, all RNG states, focus/rephase state, counters, pending mass, and runtime indexes.

## Next decision

After native validation, the owner should run a representative RAIN-004 morphology session, invoke `testingcheats classic rainmetrics`, and return the report. Those measured scales become evidence for RAIN-005B design.

RAIN-005B may replace current magic morphology constants only if its candidate is tested against RAIN-004 as the reference and targets **equal-or-greater heterogeneity**, rather than preserving RAIN-004 merely because it is currently good.
