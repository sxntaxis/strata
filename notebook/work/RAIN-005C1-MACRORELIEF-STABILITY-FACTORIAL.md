---
id: RAIN-005C1
kind: work
state: complete
created: 2026-09-08
updated: 2026-09-08
authority: working
summary: Native unattended C1 completed; convexity-route was the sole unique full human-eligible finalist, improving every coarse relief/curvature objective while remaining avalanche-safe.
---

# RAIN-005C1 — macrorelief stability factorial

## Owner target

RAIN-005B2R1 is machine-green and is the new best-in-class morphology baseline, but the owner still prefers four visible properties of the installed historical Strata: **large-scale relief, crests, valleys, and strata that thicken and laterally pinch out**. Other visual differences are not objectives for this unit.

The installed binary SHA256 `b6f3af5247ce633b4c01c6232c1f1be057f7f9af562b6a5114f424b5f3559f93` is no longer merely guessed to be 0.7.6: the durable Notebook already records that exact SHA as the real-profile installation produced by the published H4 v5 cutover (`67ffd84d3c5c924211ac9a14b52b5749fb07ed8b`). H4 is therefore the historical visual reference, although its avalanche behavior is owner-rejected and is not a runtime candidate.

## New factual correction before the experiment

Current `momentum-grounded-contact` local repose is refreshed **because diagonal movement touches a column**; it is not a wall-clock/tick resampler. The earlier idea “persist until movement” would therefore duplicate or even weaken current semantics rather than test longer persistence. C1 replaces that redundant arm with explicit stronger persistence and budget-preserving structural routing.

## Frozen runtime

Production behavior remains exact RAIN-005B2R1. All stability arms below are `#[cfg(test)]` only. Do not change:

- B2 golden-small focus authority (`1/phi^2` retained baseline);
- B2 low-discrepancy focused/uniform schedule;
- R1 actually-entered stratum-boundary focus avulsion;
- broad focused two-candidate kernel;
- ordinary RAIN-004 meander between boundaries;
- `momentum-grounded-contact` physical settlement law;
- free-site/FIFO ingress, scheduler, renderer, persistence, PERF-001.

## Stage-1 arms

64 exact A3 geometries, seed-slot 1, nine arms = **576 physical runs**.

1. `runtime-control` — exact R1.
2. `no-anchor` — map sampled repose 4 to 3; asks whether the rare maximum state matters.
3. `strong-tail` — map already-strong sampled repose 3/4 to 4; no new probability is introduced.
4. `anchor-locked` — a current repose-4 column is not resampled on movement refresh.
5. `strong-locked` — same for current repose 3/4.
6. `height-route` — after a normal refresh, the exact local multiset of repose+memory states over the touched column and immediate neighbors is permuted so stronger states occupy taller supported columns. No states are created/deleted and no RNG is added.
7. `convexity-route` — same budget-neutral permutation, but stronger states occupy the locally more convex supported columns (`2h_center-h_left-h_right`).
8. `convexity-route-anchor-locked` — arm 7 plus repose-4 lock.
9. `convexity-route-strong-tail` — arm 7 plus the existing strong-tail-to-anchor remap.

The routing neighborhood is the already-physical radius-one contact neighborhood. It is not a tuned morphology radius.

## Primary morphology measurements

The owner target supersedes shift/span/corr as the primary selection vocabulary for this unit. C1 measures:

- `relief_d2`, `relief_d4`, `relief_d8`: normalized range of supported surface height after partitioning the active width into 2/4/8 contiguous dyadic blocks;
- `curvature_d4`, `curvature_d8`: normalized absolute second difference of those coarse block means, measuring broad crests/valleys without a prominence threshold;
- `thickness_cv`: existing mean stratal thickness CV;
- `pinchout`: mean fraction of active-width columns where a stratum has zero thickness;
- `continuity`: largest contiguous nonzero run / total nonzero support for each stratum, averaged, so pinch-out is not rewarded merely by fragmentation.

The dyadic block counts are diagnostic scales only and never enter runtime.

Legacy corr/TV/shift/span remain printed for continuity, not as sole selectors.

## Avalanche-preservation diagnostics

C1 observes completed one-second inter-ingress intervals without changing the scheduler. For each run it reports diagonal moves and the min/max x touched by diagonal motion, deriving:

- mean and p95 diagonal moves per completed ingress interval;
- maximum moves;
- fraction of intervals with more than one diagonal move;
- p95/max lateral movement span;
- fraction with span greater than one column;
- pending fraction;
- exact `physical + pending == generated` mass conservation.

An arm is avalanche-safe only if cascades and multi-column movement remain semantically present and its p95/tail/frequency measures are not jointly worse than R1 control. Pending fraction may not exceed control.

## Automatic selection; no human back-and-forth

The long ignored probe performs both stages in one invocation.

Stage 1 constructs the safe Pareto frontier over the eight primary morphology objectives. It chooses at most two finalists:

- **simple finalist**: lowest mechanism count on the safe frontier, tie-broken by the best worst rank across all eight morphology objectives;
- **macro finalist**: best worst rank across the five coarse relief/curvature objectives, tie-broken by mechanism count.

No weighted score or tuned coefficient is used.

Stage 2 automatically runs exact full A3 evidence for R1 control plus the unique finalist(s): 96 geometries x 2 exact seeds x up to 3 arms = at most **576 physical runs**. A non-control full result reaches human eligibility only if it is avalanche-safe, remains on the full morphology Pareto frontier, and improves at least one coarse relief/curvature objective over R1.

No runtime candidate is authored by this unit. Human review occurs only after the unattended machine report returns.

## Transport / provenance

Parent must be exact R1 native-green return HEAD `83fde2f50ab3ada57159666ddcbdd9d380ff9baf` / TREE `c7c66e6e52bf016e1a2fc009f0caf13c3bf74752`.

C1 also adds behavior-neutral CLI provenance: `strata --version` reports Cargo semver plus the build-time Git commit short hash. This is separate from the test-only morphology experiment and exists so future installed binaries can identify their source lineage directly.
## Native result — 2026-09-08

The exact C1 handoff passed transport, scope, fixtures, compile, focused C1 gates, R1/B2 preservation, sample-level R1 control reproduction, formatting, strict Clippy, full tests, help/version smoke and PERF-001. The complete unattended suite ran for `7858.54` seconds.

Stage 1 completed `64 geometries x 1 seed x 9 arms = 576` physical runs. Every arm except `strong-locked` remained on the safe Pareto frontier, but the predeclared simple and macro selection rules independently chose the same unique finalist: `convexity-route`.

Stage 2 therefore ran exact full evidence for R1 control and that finalist only: `192 + 192 = 384` physical runs. Full medians were:

```text
runtime-control:  relief_d2=0.090879794 relief_d4=0.250406504 relief_d8=0.341726837 curvature_d4=0.140056022 curvature_d8=0.073316283 thickness_cv=0.344785759 pinchout=0.001529052 continuity=0.979844961 avalanche_safe=true
convexity-route:  relief_d2=0.095018506 relief_d4=0.254031588 relief_d8=0.349065791 curvature_d4=0.143704758 curvature_d8=0.078469649 thickness_cv=0.354764624 pinchout=0.001851852 continuity=0.972854291 avalanche_safe=true
```

`convexity-route` remained on the full safe Pareto frontier and was the sole human-eligible arm. C1 therefore closes the broad stability search: do not tune anchor probabilities, lock durations, or structural-routing radii. The next bounded unit is RAIN-005C1R1, which promotes exactly the selected budget-neutral radius-one convexity permutation to runtime and requires exact 192-row promotion reproduction before human review.
