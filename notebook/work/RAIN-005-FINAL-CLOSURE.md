---
id: RAIN-005-FINAL
kind: work
state: complete
created: 2026-09-09
updated: 2026-09-09
authority: working
summary: Close the Classic morphology search on owner-selected C2R2 anchor-apex-latent and record why the later geography families do not supersede it.
---

# RAIN-005 — final Classic morphology closure

## Decision

The owner accepts **C2R2 `anchor-apex-latent` as the new Classic runtime default**. This closes the RAIN-005 morphology search.

The accepted default combines:

- B2 low-discrepancy focused/uniform scheduling at the retained golden-small focus authority;
- B2R1 stratum-boundary focus avulsion;
- C1R1 convexity routing of the existing local-repose budget;
- C2R2 apex-latent authority: a routed `repose=4` anchor behaves as `repose=3` only while it is a strict one-column apex, with full authority returning automatically once later accretion broadens or buries that apex;
- the accepted `momentum-grounded-contact` Classic avalanche behavior.

No new timer, probability, latent-state vector, viewport radius, or extra RNG draw is introduced by C2R2.

## Human evidence

C1R1 `convexity-route` was already owner-accepted as a small but clearly visible improvement in large-scale relief, crests/valleys, and lateral stratal variation. C2R2 was then compared directly against C1R1. The owner selected C2R2 because the narrow 1–3-dot pikes read cleaner while the desired macro-topography and avalanche behavior remained visually preserved.

The owner explicitly accepts the tiny measured thickness-CV reduction because the narrow pikes themselves can contribute disproportionate mathematical variance without equivalent visual value.

## Machine evidence carried into closure

C2R2's diagnostic arm remained avalanche-safe. Later full 192-sample G2/G2R1 control reproductions reproduced the C2R2 baseline at 9 decimals before comparing geography alternatives. Ordinary closure on those descendant suites also passed formatting, strict Clippy, full library/integration tests, and PERF-001.

The final G2R1 comparison reproduced:

- C2R2/A stratal span: `0.122676053`;
- heritage-walk/B stratal span: `0.033839168`;
- heritage-walk + boundary-avulsion/C stratal span: `0.120993063`;
- C2R2/A d8 relief: `0.349889060`;
- heritage-walk/B d8 relief: `0.600732601`;
- hybrid/C d8 relief: `0.357244825`.

Thus the final hybrid recovered `98.1055278%` of the A-side stratal-span loss but only `2.9324118%` of B's A→B d8 relief gain. It also slightly worsened some curvature/crest/pike measures and was rejected without a human candidate.

## Rejected final directions

- Additional shoulder/anchor/repose routing: either negative or traded away accepted heterogeneity.
- Avalanche-toe and mesoscale geography routing: no robust full-ensemble improvement.
- Heritage H4-style persistent walk: large relief improvement but unacceptable monomountain / stratal-span collapse in the square viewport.
- Heritage walk + C2R2 boundary avulsion: restored lateral span but removed almost all of the heritage relief gain.

These are closed evidence, not pending tuning opportunities.

## Freeze rule

Treat C2R2 as the default Classic morphology. Do not reopen RAIN-005 rain/geography/repose tuning unless the owner explicitly requests it. Future work should build on this runtime rather than silently changing its morphology.
