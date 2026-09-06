---
id: CLASSIC-002
kind: work
state: candidate
created: 2026-09-05
updated: 2026-09-05
authority: working
summary: Preserve the simple Classic cellular avalanche while adding weak deterministic per-column repose heterogeneity so the final exposed slope retains small kinks and terraces instead of converging almost perfectly to a straight line.
---

# CLASSIC-002 — Local repose heterogeneity

## Owner observation

CLASSIC-001 recovered the desired macro behavior with dramatically less machinery than the Oslo/SEDIMENT experiments. The rainbow wall collapse is continuous, legible, stratified, and close to the desired result. The remaining visible defect is narrow: after relaxation, the exposed mountain surface is too linear and lacks the mild heterogeneous texture seen in the better Oslo outcomes.

The owner explicitly prefers improving Classic rather than importing the Oslo/SEDIMENT rendering or transport architecture.

## Bounded semantic change

Classic keeps its existing cellular law:

1. fall straight down when the cell below is free;
2. otherwise choose exactly one random diagonal;
3. if that chosen diagonal is blocked, wait for a later gravity pass;
4. never cross the visible closed walls;
5. render the authoritative grid directly.

CLASSIC-002 adds one local state per canonical column:

```text
local_repose[x] ∈ {1, 2}
```

`1` is exactly the CLASSIC-001 behavior. A rare `2` column tolerates one additional unit of local relief before the chosen diagonal may release. The normal candidate distribution is deliberately weak: repose `2` is sampled one time in twenty (5%); the remaining 95% is repose `1`.

The repose RNG is deterministic and independent from the existing Classic left/right physics RNG and rain RNG. Therefore the new heterogeneity does not perturb those random streams merely by being sampled.

After a successful diagonal transfer, the source and destination columns resample their local repose. A blocked high-repose site retains its local state until a later surface-changing transfer affects that column. Existing repose state follows canonical terrain when the logical canvas expands; newly exposed canonical columns receive deterministic new samples.

`clear` / `testingcheats fill` reset the repose RNG to its sandbox seed boundary before rebuilding the local field. No persistence/schema authority is introduced.

## Why only 5% high repose

The intent is not to make Classic behave like Oslo. The added state exists only to prevent thousands of independent diagonal decisions from averaging into an almost perfectly linear final slope. A weak minority of locally more-stable columns is sufficient to create short shoulders, duplicate-height steps, and small kinks while leaving the broad runout and apex character near the original Classic result.

## Explicit non-goals

- no Oslo threshold/front/rolling mechanics;
- no visual shadow, MotionId, replay, custody, parcels, or presentation physics;
- no renderer change;
- no new persistence or schema;
- no change to Fixture category creation or rainbow-fill geometry;
- no 64x-specific behavior;
- no production SandEngine promotion in this unit.

`classic` and `hybrid` continue to share the same gravity/repose law; `hybrid` remains an ingress-only comparison.

## Authored regressions

CLASSIC-002 adds focused tests proving:

- repose `1` preserves the original two-unit-relief diagonal release while repose `2` blocks exactly that marginal release;
- fixed seed + fixed fixture produces deterministic final grid, repose field, RNG state, and movement counts;
- against a test-only uniform-repose Classic control, weak heterogeneity increases exposed-profile second-difference roughness while conserving exact mass;
- apex remains within one dot of the control and footprint/runout remains within a 10% tolerance;
- canonical horizontal growth shifts existing repose state with the terrain rather than reassigning it;
- Classic and Hybrid still have identical gravity/repose behavior for identical non-ingress state.

Existing CLASSIC-001 fill/category/wall/resize tests remain required unchanged.

## Native gate

The candidate is not accepted until the local validator confirms focused CLASSIC-002 tests, all Classic tests, Fixture category idempotence/persistence, Oslo/SEDIMENT controls, fmt, strict Clippy, full tests, help smoke, and command parser.

After machine green, the owner should dogfood exactly the same `testingcheats fill` collapse and judge one question: does the final surface gain mild heterogeneous texture without losing Classic's simple avalanche character?
