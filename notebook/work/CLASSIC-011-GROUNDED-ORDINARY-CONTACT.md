---
id: CLASSIC-011
kind: work
state: completed
created: 2026-09-06
updated: 2026-09-06
authority: working
summary: Preserve momentum-repose-contact as the owner-preferred control and add one diagnostic grounded-contact profile in which an occupied cell directly below may trigger the ordinary Classic diagonal only when that blocker is bottom-connected pile; catching another airborne grain waits in the same lane for the current gravity sweep.
---

# CLASSIC-011 — Grounded ordinary contact

## Owner evidence

CLASSIC-010 is machine-green. Human review still prefers `momentum-repose-contact` over `momentum-surface`; the receiving-support bonus gate did not address the remaining visible defect closely enough.

The owner identified a more specific physics artifact during fresh `testingcheats fillhalf`: an airborne grain can appear several columns outside the falling wall even though the current frame shows open air beneath it. The desired interpretation is that free fall should remain vertical until the grain actually reaches the pile; contact with another grain that is itself falling should not create a new lateral lane.

`anchored` remains the non-momentum baseline. `momentum-repose-contact` remains the closest accepted comparison control. No existing profile is changed in this unit.

## Source diagnosis

Classic currently distinguishes only empty versus occupied in the direct-below cell for the **ordinary** diagonal:

```text
below empty
-> fall vertically

below occupied
-> choose one Classic diagonal
```

CLASSIC-009 added bottom-connectivity only for deciding whether momentum may add its optional second hop. Therefore an upper airborne grain can catch another airborne grain, take the ordinary Classic diagonal, continue falling, catch another airborne grain later, and accumulate multiple columns of lateral displacement. A static frame can then show the grain offset into open air even though each individual diagonal was triggered by a transient in-air blocker in an earlier sweep.

CLASSIC-010 changes only bonus receiving support and therefore cannot eliminate this first-diagonal mechanism.

## New profile

Add:

```text
testingcheats classic experiment momentum-grounded-contact
```

The profile is exactly `momentum-repose-contact` except for one earlier contact rule.

When the direct-below cell is occupied:

1. determine whether that direct blocker is a contiguous occupied column through the visible bottom boundary, using the existing CLASSIC-009 grounded-blocker predicate;
2. if grounded, proceed with ordinary Classic diagonal selection exactly as `momentum-repose-contact` does;
3. if not grounded, treat the event as catching another airborne grain and **wait in the same cell for this gravity sweep**;
4. an airborne wait consumes no diagonal-direction RNG draw and creates no lateral movement;
5. on a later sweep, if the blocker has fallen away, the upper grain resumes ordinary straight-down free fall.

After genuine grounded pile contact, preserve `momentum-repose-contact` exactly:

```text
grounded direct blocker
-> ordinary Classic one-side diagonal
-> repose-relative D in [R, R+1]
-> direct-blocker contact gate already satisfied
-> at most one same-direction bonus hop
```

The CLASSIC-010 receiving-support gate is **not** composed into this profile. That remains isolated in `momentum-surface` so the owner can distinguish the two hypotheses.

## Hard boundaries

Do not change:

- `anchored` default semantics;
- `momentum`, `momentum-repose`, `momentum-tangent`, `momentum-soft`, `momentum-contact`, `momentum-repose-contact`, or `momentum-surface`;
- straight-down free fall when the direct-below cell is empty;
- one-side-only Classic diagonal semantics after genuine pile contact;
- local repose, memory, slope bias, anchors, texture, or RNG streams for existing profiles;
- maximum one bonus hop;
- fill/fillhalf, renderer, categories, Oslo, persistence, schema, provenance, or production H4 semantics.

Do not add persistent airborne/settled state, velocity, trajectories, rolling grains, fronts, multi-hop loops, or two-phase physics.

## Required proof

Focused tests must prove:

- the existing `momentum-repose-contact` control still takes one ordinary diagonal when its direct blocker is airborne while suppressing only the bonus;
- `momentum-grounded-contact` leaves the upper grain in the same lane on the same airborne-blocker fixture;
- that wait preserves exact mass, performs zero diagonals, and does not consume a direction RNG draw;
- a bottom-connected direct blocker still permits the ordinary diagonal and repose-relative one-hop momentum;
- parser/profile selection accepts the new profile and rejects unknown values;
- all prior Classic profiles, fillhalf, Oslo, provenance, category, and repository regressions remain green.

Capture a `CLASSIC_011_METRICS` line for the new profile, but the human gate remains authoritative for the lane artifact.

## Human comparison

Use fresh half fills:

```text
testingcheats classic experiment momentum-repose-contact
testingcheats fillhalf

testingcheats classic experiment momentum-grounded-contact
testingcheats fillhalf
```

Judge two things separately:

1. **falling wall / free air** — grains should no longer branch laterally merely because they catch another falling grain;
2. **mature pile slope** — the entertaining repose-relative momentum avalanche should remain.

The desired result is vertical free fall until true pile contact, followed by the same local Classic avalanche law already preferred in `momentum-repose-contact`.

## Closure

CLASSIC-011 passed native validation with 431 tests, strict Clippy, formatting, parser/help, all focused grounded-contact gates, and no mechanical fixes. Human review selected `momentum-grounded-contact` as practically perfect and promoted it to the preferred Classic baseline. The only observed regression is computational: the repeated bottom-connectivity scan can saturate roughly one CPU thread during dense `fillhalf` relaxation and slow visual cadence. Physics is accepted; CLASSIC-012 owns behavior-preserving optimization of that predicate and the default promotion.
