---
id: CLASSIC-007
kind: work
state: candidate
created: 2026-09-06
updated: 2026-09-06
authority: working
summary: Refine only Classic momentum bonus-hop eligibility so the accepted one-hop continuation remains on mature slope-like relief while deep near-vertical early-fill cliffs fall back to the ordinary Classic diagonal.
---

# CLASSIC-007 — Momentum cliff gate

## Owner evidence

CLASSIC-006 is machine-green and the owner human-smoked `testingcheats fillhalf` successfully.

The CLASSIC-005/006 visual comparison established:

- `anchored` remains the preferred default Classic experiment;
- `momentum` makes mature avalanches noticeably more entertaining and coherent;
- the defect is localized to very steep / near-vertical relief, especially at the early edge of a fresh fill, where the second same-direction hop can read as a grain spawning sideways;
- once the surface develops ordinary diagonal relief, the one-hop continuation looks natural and should be preserved.

The owner suspects local slope geometry is the relevant discriminator, but did not require a literal angle or exact 45-degree solver.

## Bounded design

Do not add another transport model. Keep the existing `momentum` experiment and change only the eligibility of its already-bounded bonus hop.

After an ordinary successful Classic diagonal:

```text
ordinary diagonal succeeds
        ↓
consider existing same-direction bonus hop
        ↓
measure open drop depth in the prospective bonus column
        ↓
depth 2..=3  → bonus remains eligible
depth < 2     → existing insufficient-relief stop
depth > 3     → new cliff stop
```

`diagonal_drop_depth` is the existing local, bounded dot-grid relief probe. This is deliberately a discrete local geometry gate, not a geometric-angle calculation.

The retained lower bound (`2`) preserves the accepted CLASSIC-005 momentum trigger. CLASSIC-007 adds only the upper bound (`3`) so deep open walls no longer authorize the visually lateral-looking second hop.

## Hard boundaries

CLASSIC-007 must not change:

- default experiment (`anchored`);
- ordinary Classic diagonal behavior;
- steep-side 75% choice semantics;
- repose memory or anchor sampling;
- the maximum of one bonus hop;
- RNG policy or add new RNG consumption;
- persistent velocity, rolling state, multi-hop loops, trajectory state, front machinery, or global slope calculation;
- `fill` / `fillhalf` geometry or Fixture category semantics;
- Oslo, renderer, persistence, schema, provenance, category contracts, or production H4 semantics.

## Required proofs

Machine validation must prove:

1. `anchored` remains the default Classic experiment.
2. `momentum` still performs exactly one bonus same-direction diagonal when the prospective receiving column has local drop depth 2 or 3 and the ordinary Classic/repose checks permit it.
3. A deep/cliff-like receiving column suppresses only the bonus hop; the already-valid ordinary Classic diagonal still happens.
4. No second bonus or loop exists.
5. Exact mass conservation remains true.
6. CLASSIC-005 experiment ladder, CLASSIC-006 `fillhalf`, Classic/Hybrid/Oslo regressions, parser/help, Fixture idempotence, formatting, strict Clippy, full tests, and help smoke remain green.

## Human gate

Compare only:

```text
testingcheats classic experiment anchored
testingcheats fillhalf

testingcheats classic experiment momentum
testingcheats fillhalf
```

For `momentum`, inspect two phases:

1. **early fill / steep wall** — the second hop should no longer read as lateral spawning;
2. **mature slope** — avalanches should retain the entertaining coherent continuation that motivated keeping momentum.

Do not promote `momentum` to the default in this pass. Promotion, if desired, is a separate owner decision after this human gate.
