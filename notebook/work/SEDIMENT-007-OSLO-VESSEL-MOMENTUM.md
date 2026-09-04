---
id: SEDIMENT-007
kind: work
state: active
created: 2026-09-03
updated: 2026-09-03
authority: working
summary: Preserve oslo-vessel exactly during ordinary quiescent driving, but let a catastrophic local reconnection slope release grains into a transient causal rolling phase with lower dynamic friction.
---

# SEDIMENT-007 — Oslo-vessel causal momentum

## Owner evidence entering this unit

The owner continues to prefer `oslo-vessel` over all prior sediment experiments for ordinary topography. Its remaining defect is the large wall/reconnection event after a horizontal viewport expansion: compared with Classic, the exposed wall still relaxes too locally/serially instead of feeling like a body of sand entering motion.

Two negative experiments now constrain the next design:

- WaveView (`d22fabe9b9bffdda93fea539a325f527bd4faca3`) preserved exact vessel physics while consuming more topplings per visual frame. The owner rejected it because faster presentation did not create collective physical motion.
- Inertia v1 (`9d06deff9f9a3ca49c81d3e7ac7dc1eb328b9d3d`) waited for an avalanche to become at least `4 × visible width` in moves and about one-third-width in span before lowering friction. Owner resize evidence showed only a small seam relaxation: the event stopped before the global gate could ever engage. The gate was therefore circular for the phenomenon it was supposed to create.

SEDIMENT-007 returns to the pre-WaveView/pre-inertia vessel baseline `ad5b4c1eb0fc653f7fffdf418081d8270e566564` and changes the physical mechanism rather than its thresholds.

Production H4/v5 remains untouched. This unit is debug-only and never persisted.

## Core distinction: static versus already-moving material

The new hypothesis is a local static/dynamic-friction hysteresis:

```text
static Oslo surface
        ↓
ordinary local relief
→ exact oslo-vessel

catastrophic local relief
        ↓
Oslo releases a top grain
        ↓
that grain becomes moving material
        ↓
continues down-slope / briefly coasts on level terrain
        ↓
settles back into the static Oslo heightfield
```

This is closer to the physical distinction discussed in inertial/rolling-layer granular models than a global avalanche-size latch. It does not claim to reproduce a specific paper numerically.

## Why the trigger is local relief 4

The static Oslo threshold is always `{1,2}`. A quiescent ordinary vessel therefore has local relief no greater than its threshold. A single normal rain grain can raise that relief by at most one before relaxation, so ordinary quiescent drive can reach at most relief `3`.

Therefore:

```text
MOMENTUM_TRIGGER_RELIEF = 4
```

is not an arbitrary "large avalanche" counter. It cleanly separates ordinary one-grain Oslo evolution from a structural discontinuity that ordinary driving cannot create by itself.

The expected source of relief `>= 4` is precisely the owner-visible case: removing a temporary visible-window wall reconnects two pieces of canonical terrain that evolved independently.

A machine gate requires long ordinary matched driving to remain exact `oslo-vessel` with zero momentum seeds.

## Moving-grain phase

When a normal Oslo topple chooses an **internal** destination with selected downhill relief at least `4`:

1. the source top grain is removed exactly as in Oslo;
2. the source redraws its Oslo threshold exactly as normal;
3. instead of immediately becoming settled mass in the neighbor column, that specific grain enters a `RollingGrain` phase at the neighbor site;
4. source/destination neighborhoods remain scheduled for static Oslo relaxation.

A rolling grain owns:

- `CategoryId` — identity/color travels with the physical grain;
- current lattice site;
- its original downhill direction;
- a small flat-terrain coast budget.

### Dynamic motion law

Each physics tick, every currently rolling grain advances by at most one lattice site.

- If the next site in its direction has a **lower** settled surface, it moves there and refreshes its flat coast allowance.
- If the next site is **level**, it may continue for at most two sites before stopping.
- If the next site is **higher**, or the visible vessel wall blocks the direction, it settles at its current site.
- Settling inserts the grain back into the Oslo column and enqueues that neighborhood, so the deposited moving material can causally trigger further static instability.

This gives already-moving material a lower effective stopping friction without globally lowering the Oslo critical slope or converting every static grain into a mobile grain.

## Why many grains can move together

The rolling phase is separate from the static heightfield. A steep source may continue to fail on later physics ticks while previously released grains are still rolling. Therefore multiple physical grains can coexist in the moving phase:

```text
wall source:       grain A released → rolls → rolls
next tick:         grain B released → rolls
next tick:         grain C released

visible result:    A  →  →
                     B  →
                       C
```

This is not WaveView batching. The grains follow a different physical path and are absent from the settled destination column until they actually stop.

## Vessel and canonical constraints

`oslo-vessel-momentum` keeps the accepted vessel architecture:

- canonical width is monotonic;
- current VW is the active horizontal basin;
- hidden settled columns remain custody/frozen while cropped;
- rain only targets the VW;
- temporary VW edges remain physical walls;
- expanding removes the temporary wall and reconnects preserved terrain;
- canonical-height vessel overflow remains the only lateral mass exit.

A rolling grain cannot cross a current visible wall. At a wall it settles. If that settlement ultimately raises an edge column above the canonical rim, ordinary vessel Oslo can subsequently discharge through its existing overflow rule.

Transient rolling grains are projected into a newly shrunken visible basin in the same debug-harness spirit as in-flight rain grains; no production persistence semantics are introduced.

## Quiescence and ingress ordering

A new falling rain grain may be visible while an avalanche is active, as in the existing harness, but it cannot commit into the settled Oslo heightfield until both are empty:

- static active-site queue;
- rolling-grain queue.

An avalanche is not finalized while any rolling grain remains mobile.

## Rendering

Rolling grains are rendered as transient dots immediately above the settled surface at their current lattice site. Multiple rolling grains may be visible simultaneously. They also contribute to total current mass accounting while mobile.

No WaveView-style increase in static topplings-per-render is added.

## Diagnostics

`testingcheats status` for `oslo-vessel-momentum` exposes:

- currently rolling grains;
- total momentum seeds;
- rolling hops;
- settlements back into Oslo;
- peak simultaneous rolling grains;
- seed/hop/peak counts for the last completed avalanche.

These are debug evidence only.

## Machine gates

The candidate must prove:

1. a sub-trigger ordinary topple is state/RNG-equivalent to `oslo-vessel`;
2. 2,000 ordinary quiescent drives remain exact vessel behavior and produce zero momentum seeds;
3. relief `4` immediately seeds a moving grain on the first qualifying topple — no global size/span precondition;
4. the released grain is removed from settled source mass and remains accounted while mobile;
5. rolling material moves down-slope, can coast only a bounded distance on flat terrain, and then settles;
6. a rolling grain cannot cross the current visible vessel wall;
7. a horizontal shrink/re-expand fixture can seed momentum on the first steep reconnected seam topple;
8. a deterministic wall-step fixture creates more than one concurrently mobile grain, reaches quiescence, conserves mass, and does not leak laterally below the vessel rim;
9. every existing SEDIMENT-004 boundary/vessel test remains green;
10. deferred Oslo presentation batching remains equivalent for the unchanged ordinary controls;
11. full workspace validation and debug command parsing remain green.

## Human A/B

Compare only:

```text
testingcheats model oslo-vessel
testingcheats model oslo-vessel-momentum
```

First run both at ordinary matched age. Momentum is a failure if everyday rain/topography visibly diverges from vessel behavior before a structural reconnection.

Then perform the discriminating resize test:

1. develop a tall vessel surface in a substantially narrower VW;
2. let it become quiescent;
3. expand horizontally in one large step;
4. issue no additional `advance` until the resulting event settles;
5. watch whether the seam releases a visible moving layer rather than only serialized local topplings;
6. inspect `testingcheats status` after settlement.

Desired evidence:

- `momentum seeds > 0` immediately from the steep seam;
- `peak` simultaneous rolling grains greater than one on the large collapse;
- a visibly broad moving front / wall failure;
- individual moving dots remain legible rather than teleporting to the final state;
- the post-event surface returns to useful Oslo-vessel character;
- ordinary small activity before/after the event remains Oslo-like.

If this still fails to create a convincing mass collapse, the next distinct experiment should stop modifying the Oslo toppling law and implement a fuller static/rolling-layer model (BCRE-like) rather than another global gate or render scheduler.
