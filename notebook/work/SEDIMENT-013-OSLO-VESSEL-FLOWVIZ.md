---
id: SEDIMENT-013
kind: work
state: active
created: 2026-09-04
updated: 2026-09-04
authority: working
summary: Replace rejected per-grain transport2d presentation with a flux-driven tracer visualization over frozen SEDIMENT-008 oslo-vessel-front physics.
---

# SEDIMENT-013 — Oslo Vessel Flow Visualization

## Owner evidence

The owner rejected the SEDIMENT-012 `transport2d` presentation after rainbow-fill resize testing. The physical front still appeared useful, but the visual transport produced artificial skirts/steps and organized trajectories. The failure is classified as a presentation-model error, not a new physics rejection.

SEDIMENT-010, 011, and 012 collectively establish three negative results:

1. **global visual convergence barrier** — physically safe but serialized the collapse behind one dot;
2. **future-X + past-Y interpolation** — kept throughput but produced diagonal ray/fountain artifacts;
3. **per-grain FIFO destination paths** — geometrically coherent at the token level but still invented a Lagrangian path that the Oslo/front solver never actually specifies.

The owner explicitly requested a web/literature pass for a better transport visualization. The resulting direction is to separate the authoritative granular solver from its transport visualization instead of treating every Oslo transfer as a known grain trajectory.

## Frozen authority

SEDIMENT-008 `oslo-vessel-front` remains the owner-frozen debug physics baseline.

SEDIMENT-013 does **not** tune:

```text
Oslo static thresholds           {1,2}
catastrophic front seed relief   4
moving-layer erosion relief      2
support-loss recruitment relief  3
front coast/runout rules         unchanged
vessel/canonical boundaries      unchanged
rain/focus                       unchanged
CategoryId physical custody      unchanged
```

Production H4/v5 remains untouched. Nothing in this unit is persisted.

## Research-derived visualization model

The implementation borrows only the architectural separation common to particle/grid and granular-flow visualization techniques:

```text
AUTHORITATIVE FRONT PHYSICS
        |
        | adjacent mass transfers
        v
SIGNED EDGE FLUX FIELD
        |
        | direction + activity density
        v
BOUNDED VISUAL TRACER CLOUD
        |
        | gravity + local bed interaction
        v
TERMINAL RENDER
```

The important semantic boundary is:

```text
CategoryId grain mass = authoritative physics
FlowVizTracer           = presentation sample only
```

A tracer has a category color because it samples a real CategoryId transfer, but it is **not** a one-to-one grain identity and is excluded from all mass accounting, relief, support, thresholds, persistence, and RNG used by sediment physics.

## Why flux instead of destination paths

The front solver does know a physically meaningful local fact:

```text
one unit of CategoryId mass crossed edge x <-> x+1
```

It does not know a unique continuous ballistic path for that unit after a sequence of topplings and erosion/recruitment events.

SEDIMENT-013 therefore records only adjacent front/moving-layer transfers. Ordinary small settled Oslo topples below the severe moving-phase trigger do not emit a tracer cloud, so everyday quiescent acceleration is not trapped behind presentation work.

Each recorded transfer:

- increments a signed local edge-flux accumulator;
- may create one bounded visual tracer sample;
- initializes the tracer at the source surface elevation;
- carries the source CategoryId only for color/provenance visualization.

No future destination queue is stored.

## Edge flux

Each canonical edge has a small signed accumulator:

```text
negative = recent mass flow left
positive = recent mass flow right
magnitude = recent transfer activity
```

The field is bounded and decays every visual frame. Magnitude affects visual density/activity but **not sediment physics and not the fundamental tracer speed**. This deliberately avoids the earlier defect where large solver throughput made the avalanche appear to accelerate.

## Tracer motion

A tracer stores only transient presentation state:

```text
x, y
vx, vy
CategoryId
age/rest state
```

Movement is local:

1. above the current bed, gravity increases downward velocity with a fixed cap;
2. on contact, the recent local flux chooses the preferred lateral direction;
3. if local flux is weak, current bed relief chooses a downhill direction;
4. the tracer follows the local surface rather than aiming for a precomputed final destination;
5. if no viable flow remains, it disappears after a short rest;
6. tracers outside the visible basin or beyond a bounded lifetime disappear.

Small deterministic visual-only jitter is added at spawn so many samples of the same flux do not collapse into artificial combs. The visual RNG is isolated from every physics RNG.

## Bounded work

Tracer count is capped at 4096. When the pool is full, later transfers still update the edge-flux field but can drop **visual samples**. A dropped tracer has no mass meaning and cannot change the physical result.

Diagnostics expose:

```text
flowviz=tracers:<current>
        flux_edges:<active>
        spawned:<total>
        peak:<peak simultaneous>
        dropped:<visual samples dropped at cap>
```

The expected visual cost is bounded by current tracer count plus lattice-width flux decay rather than physical grain count times queued path length.

## Rendering contract

`oslo-vessel-front-flowviz` suppresses the rejected SEDIMENT-010/011 `visual_y` interpolation for settled and rolling grains. The authoritative settled pile remains rendered from real columns. Explicit rolling CategoryIds are represented by the tracer cloud instead of being drawn at the physically advanced x paired with a stale y.

This is intentionally a first flow-visualization experiment, not a claim that the tracer cloud is conserved dye mass. The hard invariant remains that the **underlying CategoryId physics is exactly frozen front**. Human testing decides whether flux tracers are enough to make rainbow provenance perceptually coherent before any separate conservative dye/shadow-surface layer is considered.

## Scheduler contract

SEDIMENT-011 clock separation remains:

- visible avalanche physics plays at the stable wall-clock grain cadence;
- 64x/128x accelerated synthetic debt does not make the visible avalanche run faster;
- baseline live rain remains approximately one falling dot per wall-clock second;
- already requested synthetic advance debt is preserved and resumes after visual flow ends.

FlowViz tracers count as visual motion after physical quiescence so their short tail can drain, but they do not block authoritative physics while the front is active.

## Machine gates

The local gate must prove:

1. 2,000 ordinary quiescent drives are exact frozen-front physics;
2. a visual flux transfer changes no grain/mass state;
3. one tracer visual frame moves no more than one lattice cell in either axis;
4. a deterministic wall failure produces a concurrent tracer cloud while final columns, critical slopes, physics RNG, discharge, erosion and support recruitment remain identical to frozen front;
5. the tracer cloud drains without changing physical mass;
6. existing front, clock, live-rain, fill, boundary, resize, command, full-suite and help-smoke gates stay green;
7. `oslo-vessel-front-flowviz` is debug-only and never persisted.

## Human A/B

Compare with the same rainbow-fill wall reconnection:

```text
testingcheats model oslo-vessel-front
testingcheats fill

testingcheats model oslo-vessel-front-flowviz
testingcheats fill
```

For FlowViz, judge:

- no one-by-one serialization;
- no `future-X / past-Y` ray/fountain geometry;
- no long FIFO path/comb structures;
- moving dots should read as a dense local flow cloud whose density reflects avalanche activity;
- motion should follow the current bed/slope and gravity rather than converge toward hidden endpoints;
- rainbow categories should make source provenance plausible without requiring exact per-grain identity;
- large wall collapse morphology must remain the owner-accepted SEDIMENT-008 physics;
- baseline rain remains visible;
- 64x and 128x must not change the visible avalanche speed materially.

If the tracer cloud is compelling but settled rainbow colors still appear to jump too aggressively, the next bounded experiment is a **conservative visual dye/shadow-surface advection layer** driven by the same edge flux. Do not reintroduce token identities or destination queues.
