---
id: RAIN-005C1R1
kind: work
state: candidate
created: 2026-09-08
updated: 2026-09-08
authority: working
summary: Promote C1's sole full human-eligible finalist to runtime: keep the exact R1 repose-state budget and RNG, but after a movement-triggered local repose refresh, permute the radius-one state multiset so stronger states occupy the more locally convex supported columns.
---

# RAIN-005C1R1 — Runtime convexity routing

## Selection basis

RAIN-005C1 completed its unattended 576-run stage-1 factorial and 384-run full stage-2 validation. The only unique stage-1 finalist was `convexity-route`, and it remained on the full avalanche-safe morphology Pareto frontier against exact R1 control.

Full 192-run summaries:

```text
R1 control
relief_d2      0.090879794
relief_d4      0.250406504
relief_d8      0.341726837
curvature_d4   0.140056022
curvature_d8   0.073316283
thickness_cv   0.344785759
pinchout       0.001529052
continuity     0.979844961

convexity-route
relief_d2      0.095018506
relief_d4      0.254031588
relief_d8      0.349065791
curvature_d4   0.143704758
curvature_d8   0.078469649
thickness_cv   0.354764624
pinchout       0.001851852
continuity     0.972854291
```

Both arms remained avalanche-safe. The selected arm improves every coarse relief/curvature objective and also increases stratal thickness CV and pinch-out, with a small continuity decrease that remained Pareto-safe under the frozen C1 selector.

## Runtime semantics

Production `momentum-grounded-contact` retains the exact existing repose-state distribution, texture-patch correlation, three-refresh memory, repose RNG stream, and movement-triggered refresh cadence.

The only new runtime behavior is what happens **after** an ordinary local repose refresh touches column `x`:

1. Consider the already-physical radius-one neighborhood `[x-1, x, x+1]`, clipped to the active viewport.
2. Compute each supported column's local convexity as `2*h_center - h_left - h_right`, using adjacent supported surface heights and edge self-substitution.
3. Take the exact local multiset of `(repose_state, memory_remaining)` pairs already present in that neighborhood.
4. Sort positions from less convex to more convex.
5. Sort states from weaker/shorter to stronger/longer.
6. Permute the existing states onto those positions so stronger states occupy the more locally convex columns.

No state is created or deleted. No repose probability changes. No extra RNG draw is consumed. No new persistence interval, radius, threshold, coefficient, prominence scale, or avalanche rule is introduced.

## What remains frozen

C1R1 does not change:

- B2 `1/phi^2` focused authority;
- B2 low-discrepancy anti-nozzle schedule;
- R1 actually-entered stratum-boundary focus avulsion;
- RAIN-004 broad two-candidate focus kernel and correlated meander;
- Classic `momentum-grounded-contact` diagonal release law;
- the exact 90%/8%/1.5%/0.5% anchored repose draw distribution;
- texture-patch continuation probabilities;
- three-refresh memory lifetime;
- free-site/FIFO ingress;
- mass conservation, persistence, renderer, scheduler, fallspeed, Advance, Catch Up, and PERF-001 semantics.

## Promotion-control gate

The C1 full selected-arm output is frozen as:

`tests/fixtures/rain_005c1_convexity_full_reference.csv`

- 192 data rows + header;
- SHA256 `138c979e64e046bcdb5597d2eed71b468257635a53d7caef838d21899132c6f1`.

Before human review, runtime C1R1 must reproduce all 192 selected C1 samples at nine printed decimals for:

- `relief_d2/d4/d8`;
- `curvature_d4/d8`;
- thickness CV;
- pinch-out and continuity;
- legacy corr/TV/shift/span;
- avalanche p95 moves/span.

This is a promotion-equivalence gate, not a new morphology search. If one sample differs, stop and treat the promotion as unproven; do not tune locally.

## Human gate

If promotion-equivalence, formatting, strict Clippy, full tests, help/version smoke, R1/B2 preservation and PERF-001 all pass, advance exactly this candidate to one human review. The owner judges only the target properties frozen in C1:

- large-scale relief;
- crests;
- valleys;
- strata that thicken and laterally pinch out;
- with modern avalanche behavior still visually alive.

Do not merge or push before that verdict.
