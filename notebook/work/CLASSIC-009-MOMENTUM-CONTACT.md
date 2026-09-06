---
id: CLASSIC-009
kind: work
state: candidate
created: 2026-09-06
updated: 2026-09-06
authority: working
summary: Test whether the remaining multi-lane falling-column artifact comes from momentum treating an airborne grain as a pile collision; preserve all prior controls and add fixed-band and repose-relative momentum variants that require the direct blocker to be a bottom-connected supported column before the one bonus hop is allowed.
---

# CLASSIC-009 — Momentum contact gate

## Owner evidence

CLASSIC-008 is machine-green. The owner could not reliably distinguish the A/B/C momentum gate variants and reports that none fully solves the remaining artifact: while sand is still falling, multiple lateral lanes can grow alongside a falling column, visually resembling an in-air collision that widens the stream.

`fillhalf` is accepted as a useful comparison fixture. `anchored` remains the preferred non-momentum baseline.

## Source correction to the initial wake hypothesis

Inspection of Classic's actual gravity order invalidates the first proposed **same-sweep wake marker** mechanism. Gravity scans rows bottom-up. A momentum bonus destination is lower than its source, so that destination row has already been visited; a grain processed later in the same sweep is necessarily above the source and cannot have that new bonus destination directly beneath it. A sweep-local wake marker would therefore target a collision shape the update order cannot produce and risks becoming a no-op experiment.

The source does expose a narrower real mechanism consistent with the owner's observation: Classic has no settled/airborne distinction when a source grain finds its direct-below cell occupied. An upper falling grain can therefore take the ordinary Classic diagonal after catching another still-airborne grain; momentum may then add a second same-direction diagonal and visually amplify that collision into a wider falling lane.

CLASSIC-009 tests that mechanism directly without changing ordinary Classic collision behavior.

## Profiles

Preserve every CLASSIC-008 profile exactly. Add:

```text
testingcheats classic experiment momentum-contact
testingcheats classic experiment momentum-repose-contact
```

### `momentum-contact`

Start from exact CLASSIC-007 `momentum` eligibility (`drop_depth = 2..=3`). The ordinary Classic diagonal is unchanged. The one bonus hop is additionally eligible only when the cell that originally blocked straight-down fall belongs to a contiguous occupied column through the visible bottom boundary.

In this bounded Classic representation, that is the minimal supported-pile test: a direct blocker with any air gap beneath it is treated as airborne **only for bonus-momentum eligibility**.

### `momentum-repose-contact`

Compose the same grounded-blocker requirement with CLASSIC-008 A's repose-relative gate:

```text
D in [R, R + 1]
AND
direct blocker is a grounded supported column
```

No new RNG is introduced.

## Hard boundaries

The contact gate must not alter:

- straight-down Classic motion;
- whether the ordinary one-side Classic diagonal occurs;
- `anchored` default semantics;
- any existing momentum, momentum-repose, momentum-tangent, or momentum-soft profile;
- maximum one bonus hop;
- mass, category identity, renderer, fill/fillhalf, Oslo, persistence, schema, provenance, or production H4 semantics.

No persistent airborne state, velocity, rolling grain, trajectory, front, or two-phase solver is introduced.

## Human comparison

Use fresh half fills:

```text
testingcheats classic experiment anchored
testingcheats fillhalf

testingcheats classic experiment momentum
testingcheats fillhalf

testingcheats classic experiment momentum-repose
testingcheats fillhalf

testingcheats classic experiment momentum-contact
testingcheats fillhalf

testingcheats classic experiment momentum-repose-contact
testingcheats fillhalf
```

Primary question: do the contact profiles reduce the visibly widening/multiple falling lanes while preserving the entertaining mature-slope continuation that justified momentum?
