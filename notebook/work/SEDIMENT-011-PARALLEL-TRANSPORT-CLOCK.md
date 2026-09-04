---
id: SEDIMENT-011
kind: work
state: active
created: 2026-09-04
updated: 2026-09-04
authority: working
summary: Replace SEDIMENT-010's global presentation barrier with per-grain presentation coordinates that never block frozen front physics; keep baseline live rain during avalanche playback and drain latent fluidity without misclassifying falling rain as fluid flow.
---

# SEDIMENT-011 — parallel category transport + live-rain avalanche clock

## Owner evidence entering this unit

SEDIMENT-010 exposed two regressions during the 80%-VW rainbow human test:

1. colored avalanche grains visibly descended **one by one**, making the accepted SEDIMENT-008 wall collapse look almost frozen;
2. rain appeared to stop for the duration of the special playback phase;
3. the fluid curiosity fork no longer literally serialized every dot, but remained visibly stalled/unconvincing under the same clock architecture.

The problem is not a lack of CPU. The SEDIMENT-010 scheduler globally blocked authoritative physics until every rolling grain's presentation `visual_y` converged, and it deliberately stopped generating live drive grains while preserving synthetic fast-forward debt. That solved teleportation by serializing the whole avalanche behind presentation.

This unit rejects that architecture.

## Frozen physics authority

SEDIMENT-008 `oslo-vessel-front` remains owner-frozen. SEDIMENT-009 fluid constants remain frozen.

SEDIMENT-011 does **not** tune:

- Oslo `{1,2}` thresholds;
- vessel/canonical boundaries;
- severe front seed relief `4`;
- erosion relief `2`;
- support-loss relief `3`;
- front coast/runout;
- rain/focus distribution;
- fluidity start/stop/spread/release constants;
- CategoryId mass-transfer decisions.

All changes are debug presentation/scheduling custody around that solver.

## Presentation without physics blocking

A new `column_visual_y` stack is kept one-for-one with each authoritative settled `columns[site]` stack.

When physical mass settles or topples into a new column:

```text
physics:
CategoryId enters the authoritative destination column immediately

presentation:
the same stack entry may temporarily carry visual_y = previous visible elevation
```

The order field carries no mass. `column_visual_y` carries no mass. The authoritative `columns` stack changes at the same physical event as before.

If that physical grain is moved again before its presentation catches up, the pop helper carries its **current presentation y** into the next rolling state. Thus category provenance remains continuous without postponing support/relief updates.

All presentation coordinates advance **in parallel** one visible row per effective grain frame. Physics never waits for all of them to converge.

## Three clocks, revised

### Quiescent fast-forward clock

`fallspeed` / `advance` still accumulate and drain synthetic time cooperatively while the sandbox is quiescent.

### Avalanche physics clock

Once rolling mass or queued avalanche work exists, one complete effective Oslo frame plays per stable wall-clock grain frame (historically two 32 ms internal frames = 64 ms effective visible step). The whole rolling population advances together in that frame.

### Live rain clock

During wall-clock avalanche/presentation playback, accelerated synthetic debt remains paused, but the sky no longer goes dead:

```text
1 baseline drive grain / wall-clock second
```

is launched independently of `64x/128x`.

This keeps visible rain alive without injecting hundreds of new grains into one avalanche. FIFO/quiescent deposition remains engine-owned, so in-flight dots can coexist while static drive commitment waits.

## Latent partial-fluidization

A fluidity field with:

```text
fluidity > 0
rolling == 0
active Oslo queue == empty
```

is truly latent. Falling rain does **not** make that invisible field count as visible fluid flow.

The field may therefore drain with the cooperative CPU budget until it either:

- decays to zero, or
- releases real rolling mass.

As soon as rolling mass or queued Oslo work exists, stable wall-clock avalanche playback resumes.

## Why this is different from SEDIMENT-010

SEDIMENT-010:

```text
one grain has vertical transit
→ stop all physics
→ move that presentation one row
→ repeat until every transit converges
```

SEDIMENT-011:

```text
physics frame
→ all rolling grains advance causally
→ authoritative settled stacks update immediately
→ every visible transport coordinate advances one row in parallel
→ render
```

The no-teleport requirement is retained, but the solver is no longer serialized behind it.

## Human acceptance questions

With `testingcheats fill`:

1. Does the accepted SEDIMENT-008 collapse character return?
2. Do many category colors move at once rather than one dot blocking the world?
3. Does the sky continue emitting roughly one new falling dot per second during a long avalanche?
4. At `64x` versus `128x`, does the avalanche itself keep the same visible cadence?
5. Does queued `advance` time resume only after the visible event/presentation ends?
6. Does `oslo-vessel-fluid` avoid the low-CPU apparent freeze now that latent field work ignores ordinary falling rain?

No production promotion follows from this unit.
