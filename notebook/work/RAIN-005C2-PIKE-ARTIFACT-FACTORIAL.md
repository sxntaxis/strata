---
id: RAIN-005C2
kind: work
state: candidate
created: 2026-09-08
updated: 2026-09-08
authority: working
summary: Preserve the owner-accepted C1R1 convexity-route macrorelief baseline while testing four bounded causal explanations for newly visible one-column pikes of at most three dots: delayed routed-anchor authority, stationary anchors, accretion-deferred routing, and a shoulder-aware route that refuses to reward strict one-column apices.
---

# RAIN-005C2 — Pike artifact factorial

## Owner gate entering C2

The owner visually accepted RAIN-005C1R1 `convexity-route` as the new preferred Classic morphology baseline. The gain is small but clearly visible and positive in the target properties: large-scale relief, crests/valleys, and lateral stratal variation.

During that same review, the owner identified a narrower regression: occasional surface pikes/needles of roughly one to three dots. The screenshot-level hypothesis is that a newly routed high-repose state may stabilize a fresh local crest too early. The owner explicitly asked that this hypothesis not bias the search, so C2 tests it alongside three alternative causal mechanisms.

C1R1's full 192-row promotion certification remains required before eventual freeze/merge, but it is deliberately postponed until the pike artifact is resolved or rejected. There is no value certifying a baseline that may be immediately superseded by a bounded artifact fix.

## Source diagnosis

C1R1 routes the existing radius-one `(repose, memory)` multiset after movement-triggered refresh. Routing priority is local supported-height convexity:

```text
convexity(x) = 2*h(x) - h(x-1) - h(x+1)
```

This is budget-neutral and produced the accepted macrorelief gain, but a strict one-column apex also receives a large positive convexity score. A rare `repose=4` anchor can therefore be moved onto exactly the fresh narrow peak whose survival it then helps reinforce.

C2 does **not** assume this is the only cause. It tests four one-mechanism alternatives against exact runtime C1R1.

## Test-only arms

All arms preserve C1R1 rain, B2 scheduling, R1 boundary avulsion, `momentum-grounded-contact`, the exact repose draw distribution, texture correlation, RNG streams, mass law, and renderer.

### P0 — `runtime-c1r1`

Exact current runtime. This is the control and must reproduce the frozen C1 convexity reference at nine printed decimals on every sampled geometry/seed.

### P1 — `anchor-buried-activation`

Test the owner's hypothesis as narrowly as possible without creating another timer or probability:

- convexity routing remains unchanged;
- only an anchor (`repose=4`) that is **moved by routing** onto a new column has its fourth repose unit made latent;
- while latent it behaves as `repose=3`;
- it becomes full `repose=4` only after later supported accretion grows that same column above the supported height at which the anchor was routed;
- naturally sampled anchors that were not newly moved remain unchanged.

This asks whether the artifact is specifically immediate extra authority on a fresh routed apex.

### P2 — `anchor-stationary`

Convexity routing may permute `repose=1/2/3`, but existing `repose=4` anchors stay on their current columns. The state budget and anchor count remain exact.

This isolates whether moving the rare anchor at all is the pike source, while preserving structural routing for the rest of the tail.

### P3 — `deferred-routing`

Movement-triggered repose refresh remains, but its convexity permutation is queued rather than applied immediately. The queued radius-one route is consumed only by a later supported surface-accretion event in that neighborhood.

This tests whether the problem is not anchor strength specifically, but measuring/rewarding a surface immediately after the movement that created it.

### P4 — `shoulder-route`

Keep immediate budget-neutral routing, but change only the structural rank:

- a strict one-column local apex is never preferred over a non-apex neighbor merely because its convexity is high;
- among non-apex candidates, the existing convexity score remains the ordering signal;
- no blur radius, prominence threshold, coefficient, timer, or probability is added.

This tests whether C1's structural definition is slightly wrong: stability may belong on shoulders/broad crest support rather than on the apex cell itself.

## Pike diagnostic

C1's macro metrics can accidentally reward a narrow spike as additional curvature. C2 therefore adds a test-only artifact diagnostic directly matched to the owner observation.

For the final supported-height profile, an interior column is a narrow pike when:

1. it is higher than both immediate neighbors;
2. its prominence above the higher neighbor is one, two, or three dots.

C2 reports:

- `pike_density_le3`: fraction of interior columns that are such pikes;
- `pike_excess_le3`: summed one-to-three-dot prominence divided by interior width.

The value `3` is not a runtime parameter. It is only the bounded measurement class named by the owner-visible artifact.

## Experiment geometry and parallel execution

Use 32 systematically spread A3 geometries: every third geometry from the exact 96-geometry order, with both exact frozen seeds.

```text
32 geometries x 2 seeds x 5 arms = 320 physical runs
```

The five arms are independent and the diagnostic test runs them on five scoped standard-library worker threads. This changes only validation wall time; each physical engine remains single-threaded and deterministic.

The 64 C1R1 control samples must reproduce `tests/fixtures/rain_005c1_convexity_full_reference.csv` at nine printed decimals for every field already frozen by C1R1.

## Selection

No weighted score is permitted.

1. Reject any non-control arm that fails C1 avalanche safety.
2. Compute a Pareto frontier where lower pike density/excess is better and the existing eight C1 morphology objectives remain higher-is-better.
3. A human-eligible arm must remain on that safe frontier and improve at least one paired pike metric versus exact C1R1.
4. Return at most two candidates: the frontier arm with least pike excess and, if different, the frontier arm with least pike density.

No runtime arm is promoted automatically. Human review decides whether the visible pikes are actually reduced while the accepted C1R1 macrorelief character remains.

## Stop rules

- No local semantic tuning after a failed compile/test or disappointing metric.
- No changes to anchor probability, memory count, routing radius, rain authority, avalanche law, or momentum.
- No C1R1 full 192-row certification during this diagnostic pass.
- If no pike-improving avalanche-safe frontier exists, stop and report the negative result.
- If a candidate exists, return it for owner review before authoring runtime promotion.

## Native result and closure correction — 2026-09-08

C2 completed all `320` physical runs in `591.89 s`; exact C1R1 control reproduced the frozen fixture. `shoulder-route` was the only robust paired pike suppressor, reducing aggregate median pike density/excess to `0.107142857 / 0.113095238` from `0.136904762 / 0.160377358`, with paired median deltas `-0.029411765 / -0.048780488` and avalanche safety preserved. However it also reduced thickness CV by `-0.010366352` and pinch-out by `-0.000402576`. Those are owner-target properties, so the original broad Pareto selector was too permissive for an already human-accepted quality floor. `shoulder-route` is retained as causal evidence, not promoted.

Ordinary full-test closure also stopped on `memory_profile_holds_local_repose_for_three_surface_refreshes`. This is an obsolete fixed-column assertion under C1R1, whose frozen runtime semantics intentionally permute `(repose,memory_remaining)` tuples spatially. RAIN-005C2R1 authors the semantic test correction and a narrower follow-up; the local validator was correct not to repair it autonomously.
