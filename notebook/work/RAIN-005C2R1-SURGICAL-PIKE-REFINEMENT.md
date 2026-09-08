---
id: RAIN-005C2R1
kind: work
state: candidate
created: 2026-09-08
updated: 2026-09-08
authority: working
summary: C2 showed that broad shoulder-routing clearly suppresses one-to-three-dot pikes but also gives back too much of the owner-accepted lateral stratal variation. R1 therefore tests three narrower apex/anchor mechanisms against exact C1R1, with a hard no-regression floor on every owner target metric.
---

# RAIN-005C2R1 — surgical pike refinement

## Why C2 did not close

The five-arm C2 factorial completed its 320 physical runs and found one strong artifact suppressor: `shoulder-route` reduced median pike density from `0.136904762` to `0.107142857` and pike excess from `0.160377358` to `0.113095238`, while avalanches remained alive. However, the same arm reduced thickness CV from `0.354684962` to `0.344318610` and pinch-out from `0.001851852` to `0.001449275`.

Those two metrics are direct proxies for the owner's accepted target that strata thicken, thin, and disappear laterally. C1R1 is already owner-accepted as a small but real improvement, so C2 must not erase that gain merely to smooth a local artifact. The broad shoulder arm is therefore diagnostic evidence, not a promotion candidate.

The C2 closure also exposed one ordinary test whose old assertion no longer matched the already-authored C1R1 runtime semantics. `convexity-route` deliberately transports the exact `(repose, memory_remaining)` tuple to a neighboring structural location. Three-refresh memory therefore belongs to the routed state tuple, not permanently to the original column coordinate. The test is corrected to follow the unique state through its three touched refreshes while proving the repose RNG remains unchanged until expiry. This changes no runtime behavior.

## Frozen runtime

Production remains exact owner-accepted C1R1 `convexity-route`. R1 is test-only. It does not change rain, avalanche law, repose probabilities, memory count, routing radius, RNG streams, or runtime candidate code.

## Surgical arms

Run the exact same 32 systematically spread A3 geometries, both frozen seeds, on four parallel arms:

1. `runtime-c1r1` — exact accepted control.
2. `anchor-shoulder-route` — run ordinary convexity routing first; only if a `repose=4` anchor lands on a strict one-column apex, swap that exact `(repose,memory)` tuple onto the most convex non-apex neighbor carrying a weaker state. Repose 1/2/3 routing remains exact C1R1.
3. `anchor-apex-latent` — ordinary convexity routing remains exact. A `repose=4` state on a strict one-column apex behaves as repose 3 only while that apex geometry exists; once the surface broadens or buries it, full repose 4 authority is automatic. No timer, latent-state vector, or extra RNG exists.
4. `strong-shoulder-route` — same surgical post-route apex guard as arm 2, but applies to both repose 3 and 4. This tests whether the artifact needs the whole strong tail rather than anchors alone.

The strict-apex predicate is already C2's radius-one lattice geometry. No new prominence threshold or blur radius is introduced.

## Exact owner-quality floor

No weighted score and no tolerance constant is permitted. A non-control arm is human-eligible only if all of these medians on the exact same 64 samples are **greater than or equal to C1R1**:

- relief d2;
- relief d4;
- relief d8;
- curvature d4;
- curvature d8;
- thickness CV;
- pinch-out.

It must also remain avalanche-safe and improve at least one paired median pike metric. Continuity is reported but is not an owner target and therefore is not a hard floor.

If multiple arms pass, choose the one with least pike excess, then least pike density. Return at most one human candidate. If none passes, stop: keep C1R1 and accept that this local artifact cannot be removed by these surgical anchor/apex semantics without trading away accepted macrorelief.

## Run size

```text
32 geometries x 2 seeds x 4 arms = 256 physical runs
```

The four arms run concurrently on scoped standard-library worker threads, exactly as C2 did. The exact C1R1 control samples must again reproduce the frozen 192-row convexity fixture at nine printed decimals.

## Closure

After the focused R1 probe:

- formatting;
- strict Clippy;
- ordinary full tests, including the corrected routed-state memory contract;
- C1R1 24-run smoke;
- R1/B2 preservation;
- anti-nozzle;
- PERF-001.

No runtime morphology promotion, full 192-row certification, merge, or push is authorized yet. A passing R1 candidate goes directly to owner visual review.
