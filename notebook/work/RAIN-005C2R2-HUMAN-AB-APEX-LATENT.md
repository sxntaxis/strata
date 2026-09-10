---
id: RAIN-005C2R2
kind: work
state: complete
created: 2026-09-08
updated: 2026-09-09
authority: working
summary: Owner selected anchor-apex-latent over C1R1 by eye: the narrow one-to-three-dot pikes are cleaner while the accepted large-scale relief, crests/valleys, lateral thickening/thinning, pinch-outs, and avalanche behavior remain compelling. RAIN-005 is closed with C2R2 as the Classic default.
---

# RAIN-005C2R2 — human A/B for apex-latent anchors

## Owner decision

RAIN-005C2R1 correctly rejected every pike arm under the predeclared exact no-regression floor. The owner then clarified that this floor is intentionally more conservative than the human product criterion: a very small thickness-CV regression may be acceptable if it is not visually apparent, especially because the narrow pikes themselves may inflate a purely mathematical thickness statistic more than they contribute to desired visual relief.

This does **not** authorize arbitrary tuning. It authorizes one bounded human A/B of the already-measured nearest-free-lunch arm: `anchor-apex-latent`.

## Candidate semantics

Keep exact owner-accepted C1R1 `convexity-route`, rain, RNG, repose distribution, three-refresh routed-state memory, and avalanche law.

Only effective anchor authority changes:

- a routed `repose=4` anchor retains its exact state and memory;
- while that column is a strict one-column apex relative to both immediate supported neighbors, the state behaves as `repose=3`;
- as soon as the surface broadens or buries that apex, full `repose=4` authority returns automatically;
- no timer, extra probability, new state vector, prominence threshold, blur radius, or RNG draw exists.

This is the exact C2R1 diagnostic mechanism already measured as `anchor-apex-latent`; R2 is a runtime human-review promotion only.

## Existing machine evidence

On the exact 64-sample C2R1 spread, relative to C1R1:

- pike density: `0.136904762 -> 0.133333333`;
- pike excess: `0.160377358 -> 0.154761905`;
- relief d2: unchanged;
- relief d4: `+0.002722082`;
- relief d8: `+0.000640913`;
- curvature d4: `+0.000829962`;
- curvature d8: unchanged;
- thickness CV: `-0.000580293`;
- pinch-out: `+0.000164277`;
- continuity: `+0.005727460`;
- avalanche safety: pass.

The exact hard floor failed only because thickness CV decreased slightly. Human A/B, not another machine search, now decides whether that trade is perceptually worthwhile.

## Human gate

Compare the exact accepted C1R1 binary against the exact R2 candidate under separate disposable profiles. Judge only:

1. whether narrow one-to-three-dot pikes are visibly less objectionable;
2. whether large-scale relief, crests/valleys, lateral thickening/thinning, and pinch-outs remain at least as compelling by eye;
3. whether avalanches remain natural.

If the owner prefers R2, author a final unified runtime/test promotion and perform the full 192-row certification plus ordinary closure. If not, retain exact C1R1 and stop pike work.


## Final owner decision — 2026-09-09

The owner selected **B / `anchor-apex-latent`** over C1R1 in the bounded A/B. The improvement is deliberately small: narrow one-to-three-dot pikes read cleaner, while the large-scale relief, crests/valleys, lateral stratal variation, and natural avalanche behavior remain visually preserved. The tiny machine-side thickness-CV loss is therefore accepted as a measurement trade rather than a product regression.

Subsequent geography work did not supersede this result:

- RAIN-005G1 exhausted additional repose/geography routing; no candidate survived the full ensemble.
- RAIN-005G2 `heritage-walk-golden-small` produced a large relief gain but collapsed stratal span and read as a monomountain in the square viewport; the owner preferred C2R2.
- RAIN-005G2R1 combined the heritage walk with C2R2 boundary avulsion. It recovered `98.1055278%` of the lost A-side stratal span but captured only `2.9324118%` of B's d8 relief gain and slightly worsened several crest/pike metrics. It was rejected machine-side with no human candidate.

### Frozen default

Classic morphology is now frozen on the C2R2 semantics:

- RAIN-005B2 low-discrepancy focused/uniform scheduling with the retained golden-small authority;
- RAIN-005B2R1 stratum-boundary focus avulsion;
- C1R1 convexity routing of the existing local-repose budget;
- C2R2 apex-latent anchor authority on strict one-column apices;
- the accepted modern `momentum-grounded-contact` avalanche behavior.

No additional rain/geography/repose morphology search is authorized by this closure. Reopen RAIN-005 only on an explicit owner request.
