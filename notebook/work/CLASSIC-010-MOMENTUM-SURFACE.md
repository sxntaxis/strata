---
id: CLASSIC-010
kind: work
state: candidate
created: 2026-09-06
updated: 2026-09-06
authority: working
summary: Preserve CLASSIC-009 controls and add one focused momentum-surface profile that composes the owner-preferred repose/contact gate with a grounded receiving-support requirement, so bonus momentum may continue only toward bottom-connected pile relief rather than an airborne pseudo-surface.
---

# CLASSIC-010 — Momentum surface gate

## Owner evidence

CLASSIC-009 is machine-green. In human comparison the owner was using `momentum-repose-contact` and still observed occasional lateral points whose receiving column was not complete to the bottom. The direct-blocker contact gate visibly improved the earlier multi-lane artifact, but did not fully remove unsupported-looking lateral growth.

`fillhalf` remains accepted as the comparison fixture. `anchored` remains the non-momentum default/control. Existing CLASSIC-007/008/009 experiment profiles must remain unchanged.

## Source diagnosis

CLASSIC-009 answers only the first contact question:

> Did the original vertical fall stop on bottom-connected pile rather than another airborne grain?

After that check passes, the ordinary Classic diagonal occurs and the bonus candidate is gated by repose-relative drop depth. The bonus destination's first occupied support below it is not checked for bottom connectivity. Therefore the bonus can still follow relief defined by an airborne grain or airborne mini-stack.

This does not violate CLASSIC-009; it is outside that gate's scope.

## New profile

Add:

```text
testingcheats classic experiment momentum-surface
```

`momentum-surface` is exactly `momentum-repose-contact` plus one receiving-surface condition:

1. original direct blocker must be bottom-connected, as in CLASSIC-009;
2. ordinary Classic diagonal remains unchanged;
3. repose-relative bonus gate remains `D in [R, R + 1]`;
4. find the first occupied cell strictly below the proposed bonus destination in that receiving column;
5. the bonus is allowed only if that first support is itself contiguous through the visible bottom boundary.

No support below the proposed bonus destination means no bonus.

The receiving-support check controls only the optional second diagonal. It must not suppress the ordinary Classic diagonal.

## Hard boundaries

Do not change:

- `anchored` default semantics;
- `momentum`, `momentum-repose`, `momentum-tangent`, `momentum-soft`, `momentum-contact`, or `momentum-repose-contact`;
- straight-down motion or ordinary one-diagonal Classic behavior;
- the maximum-one-bonus-hop rule;
- repose sampling, memory, slope bias, anchors, or texture;
- fill/fillhalf, renderer, categories, Oslo, persistence, schema, provenance, or production H4 semantics.

Do not add persistent airborne state, velocity, rolling grains, trajectories, fronts, multi-hop loops, or two-phase physics.

## Required proof

Focused tests must prove:

- CLASSIC-009 `momentum-repose-contact` still exposes the remaining case: grounded original blocker + airborne first receiving support can produce the bonus;
- `momentum-surface` preserves the ordinary diagonal but suppresses that bonus;
- `momentum-surface` retains the bonus when both the original blocker and first receiving support are bottom-connected;
- parser/profile selection accepts the new profile and rejects unknown values;
- exact mass conservation remains true;
- all previous Classic profiles and `fillhalf` regressions remain green.

## Human comparison

Use fresh half fills:

```text
testingcheats classic experiment momentum-repose-contact
testingcheats fillhalf

testingcheats classic experiment momentum-surface
testingcheats fillhalf
```

Primary question: does `momentum-surface` remove the remaining unsupported-looking lateral points / lane growth while preserving the entertaining continuation on an already formed pile slope?
