---
id: SEDIMENT-014R3
kind: work
state: active
created: 2026-09-04
updated: 2026-09-04
authority: working
summary: Repair the persistent 11 conservative FlowViz false misses by separating successful CategoryId shadow custody withdrawal from viewport-y projection; offscreen shadow mass remains real custody even when its source depth is above the current renderable height.
---

# SEDIMENT-014R3 — Custody / projection separation

## Trigger

Native validation of SEDIMENT-014R2 still ended the ordinary 2,000-drive frozen-front regression with:

```text
flowviz_shadow_misses = 11
```

R2's same-CategoryId surplus reconciliation did not change that count.

## Root cause

The remaining failure is not another fungibility case.

`withdraw_flowviz_shadow_grain(site, category)` performed two logically separate operations through one `Option<usize>` return value:

1. remove matching CategoryId mass from conservative shadow custody;
2. derive a render-space source `y` from the removed stack depth.

The implementation removed the grain first and then used:

```text
grid_height.checked_sub(depth + 1)
```

For a shadow column taller than the current viewport, a matching grain can exist at `depth >= grid_height`. In that case the CategoryId was **successfully removed**, but the y projection returned `None`.

Callers interpret `None` as "no matching shadow custody existed". Therefore an over-height discharge could execute:

```text
real matching shadow unit removed
→ withdrawal reports None
→ parcel fallback absent
→ R2 surplus fallback sees no surplus, because it was already removed
→ flowviz_shadow_misses += 1
```

The diagnostic was false even though the mass mutation itself had already occurred.

The same conflation is potentially dangerous at the other two callers: moving-phase entry can misclassify an actual withdrawal as fungible reuse, and ordinary settled transfer can fail to mirror the already-withdrawn unit to its destination.

## R3 contract

`Option::None` from `withdraw_flowviz_shadow_grain` must mean exactly one thing:

```text
no matching CategoryId exists in that shadow site
```

Once a matching CategoryId is found and removed, withdrawal is successful regardless of whether the original stack depth lies above the current viewport.

The returned `usize` is presentation-only. For an over-height depth it is projected to the top render boundary with saturating subtraction:

```text
y = grid_height.saturating_sub(depth + 1)
```

Thus:

```text
custody truth != viewport representability
```

No physics, CategoryId identity, routing, settlement demand, or discharge policy changes.

## Why R2 remains

R2's dynamic same-category shadow-surplus fallback remains valid for genuinely fungible displacement where exact-source shadow custody and parcel custody are both absent. R3 does not broaden or weaken that rule.

R3 only prevents an exact-source withdrawal that already succeeded from being misreported as absent because its source y is offscreen.

## Regression coverage

R3 adds explicit over-height fixtures for all three callers of shadow withdrawal:

1. settled discharge removes the extra shadow unit with zero miss;
2. moving-phase entry converts the over-height shadow unit into real parcel custody rather than false reuse;
3. ordinary settled transfer mirrors the over-height source unit to the destination rather than silently losing visual mass.

Each fixture requires total/category conservation after the complete physical+visual operation.

## Frozen authority

Unchanged:

- production H4/v5;
- SEDIMENT-008 `oslo-vessel-front` physics;
- Oslo thresholds `{1,2}`;
- front seed/erosion/support-loss/runout;
- rain/focus;
- vessel/canonical boundary semantics;
- physical CategoryId custody;
- R1 dynamic settlement demand;
- R1 directional transport ledger and parcel routing;
- R2 fungible discharge-surplus rule;
- persistence/schema;
- scheduler/live-rain clocks.

## Acceptance

Native validation must prove, in order:

- the exact former 11-miss ordinary regression reaches zero;
- all three new over-height custody/projection regressions pass;
- the R2 fungible-discharge-surplus regression remains green;
- all R1 delayed-deposit/re-entry/two-edge/wall-drain regressions remain green;
- full repository gates pass before any human A/B resumes.
