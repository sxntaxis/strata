---
id: CLASSIC-006
kind: work
state: candidate
created: 2026-09-06
updated: 2026-09-06
authority: working
summary: Freeze anchored as the owner-selected default Classic experiment after the CLASSIC-005 human comparison, and add a centered half-width rainbow fill cheat so avalanche inspection no longer requires terminal resizing.
---

# CLASSIC-006 — Anchored baseline + centered half fill

## Owner human gate

CLASSIC-005 machine validation passed. The owner then compared the six profiles in order with fresh rainbow fills.

Owner findings:

- `rugged` through `anchored` changed the surface only subtly; the added complexity remained compatible with Classic's preferred avalanche character;
- `anchored` is the new preferred Classic baseline;
- `momentum` is visually valuable because it makes mature-slope avalanches more entertaining and coherent, but it is not promoted yet;
- the remaining momentum defect is localized to very steep/near-vertical early-fill relief, where the bonus hop can read as lateral spawning; once the surface reaches an ordinary diagonal slope the same one-hop behavior looks natural.

CLASSIC-006 does **not** tune momentum. That experiment remains available unchanged for the next focused pass.

## Default Classic experiment

A newly constructed Classic/Hybrid sandbox now starts with:

```text
texture = rugged
experiment = anchored
```

The existing experiment selector remains available and non-retroactive:

```text
testingcheats classic experiment rugged
testingcheats classic experiment memory
testingcheats classic experiment slope
testingcheats classic experiment memory-slope
testingcheats classic experiment anchored
testingcheats classic experiment momentum
```

No persisted production authority, schema, renderer, Oslo semantics, or category contract changes.

## `testingcheats fillhalf`

Add:

```text
testingcheats fillhalf
```

It is the existing rainbow fill with one and only one geometric difference: horizontal span.

Let the current fill-visible width be `W`.

```text
H = max(1, floor(W / 2))
offset = floor((W - H) / 2)
span = [visible_start + offset, visible_start + offset + H)
```

Within that centered span, `fillhalf` preserves the current `fill` semantics exactly:

- 80% of current visible height;
- bottom anchored;
- same configured Fixture category order and layer partition;
- same category provisioning/reuse path;
- same clear/reset behavior before construction;
- no pending grains introduced by the fixture;
- no profile/experiment selection changes.

`testingcheats fill` remains full-width and unchanged.

The command is available wherever current `fill` is available: Classic, Hybrid, and Oslo sandbox models. H4 remains unsupported.

## Validation intent

Machine validation must prove:

1. default Classic experiment is `anchored` with rugged texture base;
2. all six explicit experiment selectors still work;
3. `fill` retains its exact existing full-width geometry;
4. `fillhalf` fills exactly the centered half-width interval while preserving the same 80%-height vertical layer construction;
5. odd/even visible widths use the deterministic floor rule above, with a minimum one-column fill;
6. Classic and Oslo half-fill geometry is exact;
7. Fixture category provisioning stays on the existing idempotent path;
8. parser/help expose `fillhalf`;
9. Classic/Hybrid/Oslo regressions, fmt, strict Clippy, and full tests remain green.

## Deferred CLASSIC-007 question

Tune only the `momentum` bonus-hop eligibility on cliff-like early-fill geometry. Preserve `anchored` as the control and avoid persistent velocity, rolling state, multi-hop loops, global slope calculation, or any broader transport machinery.
