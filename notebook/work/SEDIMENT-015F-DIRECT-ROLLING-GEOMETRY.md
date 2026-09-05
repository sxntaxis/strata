---
id: SEDIMENT-015F
kind: work
state: active
created: 2026-09-05
updated: 2026-09-05
authority: working
summary: Replace reconstructed rolling-carrier Y geometry with geometry assigned directly to each RollingGrain at the authoritative recruitment/hop event, while preserving 015E coherent replay, 015D truthful raster, exact unit custody, and frozen Oslo physics.
---

# SEDIMENT-015F — Direct rolling geometry

## Entry condition

SEDIMENT-015E passed its coherent semantic gates and materially improved temporal assembly, but the owner rejected the rainbow human gate.

Observed 015E evidence:

```text
- responsiveness remained acceptable;
- chronology looked better than 015D;
- detached diagonal rails / hollow ribbons still appeared in empty space;
- final stack remained plausible enough to show that the defect is primarily in transit geometry.
```

Human result:

```text
FAIL_RECONSTRUCTED_ROLLING_GEOMETRY
```

## Source diagnosis

The fields named `observed_source_y` / `observed_target_y` are immutable once recorded, but rolling Y was not actually sourced from the `RollingGrain` before and after a hop.

For continuing rolling motion, 015C-E derived geometry from:

```text
previous carrier target
+
current settled column height
+
aggregate rolling depth at destination
```

while the authoritative moving object already carries its own presentation state. 015F keeps the legacy integer `visual_y` untouched for older controls and adds an optional direct unit lane on that same object:

```text
RollingGrain {
    site,
    visual_y,          // legacy control geometry
    unit_observed_y,   // 015F direct event geometry
    category_id,
    direction,
    ...
}
```

This separate field is necessary because 015C+ permits bottom-relative offscreen Y values above the current viewport, including negative coordinates, while legacy `visual_y` is an unsigned raster coordinate. The direct field is presentation-only and exists on the same rolling grain whose physical hop is being observed.

## 015F contract

015F introduces a presentation-only direct-geometry profile extending 015E.

For a newly recruited rolling grain:

```text
source_y = exact settled top elevation at release
assigned_target_y = next rolling lane at destination
record segment(source_y -> assigned_target_y)
RollingGrain.unit_observed_y = assigned_target_y
```

For every later physical hop:

```text
observed_source_y = RollingGrain.unit_observed_y immediately before hop
assigned_target_y = destination rolling lane chosen once for this event
record segment(observed_source_y -> assigned_target_y)
RollingGrain.unit_observed_y = assigned_target_y
RollingGrain.site = destination
```

The recorded segment is thereafter immutable. Later changes to settled columns, rolling depth, shadow state, resize-visible bounds, or other carriers may not recalculate that hop's Y.

## Authority boundary

`RollingGrain.unit_observed_y` remains presentation-only. 015F must not use it for:

- downhill direction;
- relief;
- threshold/RNG decisions;
- erosion/support-loss eligibility;
- coast budget;
- settlement site;
- discharge;
- CategoryId custody.

The only production-path changes in momentum are assignment/capture of presentation Y around an already-decided physical hop.

015E remains an unchanged control constructor. `testingcheats model oslo-vessel-front-grains` selects the new 015F direct-geometry constructor.

## Preserved foundations

015F must preserve:

- 015E coherent batch barrier and one-authoritative-quantum chronology;
- 015D truthful O(N)-class raster and live-rain semantics;
- 015A one-unit MotionId / exact ordered custody;
- no future-route prediction;
- exact CategoryId mass and final stack;
- frozen SEDIMENT-008 front physics;
- persistence/schema and canonical production profile.

## Required native evidence

1. Direct constructor extends 015E with identical physical/RNG state.
2. Initial rolling recruitment segment target equals the lane actually assigned to its `RollingGrain`.
3. Multiple grains recruited to one destination receive distinct direct lanes and each segment matches its grain.
4. For a continuing hop, segment source Y equals `RollingGrain.unit_observed_y` immediately before physics moves it.
5. Segment target Y equals `RollingGrain.unit_observed_y` immediately after that hop.
6. Later column mutation cannot alter already-recorded hop geometry.
7. Frozen-front parity, exact final stack, zero unit misses, discharge, and avalanche moves remain exact.
8. All 015E, 015D, 015C, 015B, 015A and legacy conservative regressions remain green.
9. 015D/015E scaling remains green; 015F must not reintroduce PBD or search-heavy raster work.
10. fmt, strict Clippy, full tests, help smoke and command parser pass, modulo explicitly identified host-environment failures only.

## Human question

Only after machine green:

> Do the remaining detached rails/ribbons disappear when every visible rolling hop interpolates the exact Y lane assigned to that same RollingGrain at the physical event, while retaining 015E chronology, 015D responsiveness, and live rain?
