---
id: SEDIMENT-009
kind: work
state: active
created: 2026-09-04
updated: 2026-09-04
authority: working
summary: Preserve owner-frozen oslo-vessel-front physics, make explicit-flow playback stable under testing acceleration, add a rainbow 80%-VW fixture, and fork a curiosity-only partial-fluidization/order-field model.
---

# SEDIMENT-009 — stable front cadence + partial fluidization curiosity fork

## Owner evidence entering this unit

Human testing of SEDIMENT-008 produced the strongest positive result so far. The owner explicitly accepted the avalanche behavior, category distribution, and wall-collapse character and asked that model to be frozen. The remaining complaint was temporal rather than physical: during accelerated testing the collapse could become too fast or change apparent speed, no longer rhyming with the visible falling-dot cadence.

The owner also requested:

1. a debug fixture that fills 80% of the **current visible window** with horizontal category-color layers, because the rainbow stratigraphy made grain redistribution legible; and
2. a separate curiosity model showing what an explicit **rolling-layer / partial-fluidization field** could look like, without replacing or retuning the accepted SEDIMENT-008 model.

Production H4/v5 remains untouched. Everything in this unit is debug-only and non-persistent.

## Frozen baseline

The physical behavior of:

```text
testingcheats model oslo-vessel-front
```

is frozen at:

```text
8817e1990188c76b50246e6d84edcdde79a19a60
```

SEDIMENT-009 does not tune its:

- Oslo `{1,2}` thresholds;
- vessel/canonical boundary semantics;
- catastrophic relief-4 seed;
- moving-grain rules;
- erosion threshold;
- uphill support-loss threshold;
- thickness-dependent coast budget;
- rain/focus behavior;
- mass/category transfer rules.

The source now contains no-op hooks for the new fork when `fluid_enabled == false`; machine validation must prove ordinary and wall-collapse front behavior remains unchanged.

## Stable flow cadence — harness only

The earlier cooperative testing-cheats scheduler drains synthetic time according to a CPU budget. That is good for responsiveness but bad for visual constancy: when a front is cheap to compute, many physics ticks can occur between renders; when the front becomes expensive, fewer fit. Apparent avalanche velocity can therefore vary with solver cost.

SEDIMENT-009 treats this as a debug scheduler problem.

When an explicit moving phase exists (`rolling_grains` or the new fluidity field):

```text
synthetic-time debt may continue accumulating
        ↓
physics consumption is capped to one physics event
per 32 ms wall-clock interval
        ↓
engine event order is unchanged
        ↓
when the flow ends, ordinary accelerated catch-up resumes
```

`32 ms` is the existing Strata physics period. Oslo itself still advances visible grain motion every second internal frame, so the resulting motion cadence remains aligned with the existing falling-grain mechanism rather than with host CPU speed.

This changes **wall-clock presentation only**. It must not change the engine's physical event sequence or final state for the same simulated event stream.

## Rainbow fill fixture

New debug command:

```text
testingcheats fill
```

For Oslo sandbox models it:

- clears only the disposable testing sandbox;
- fills exactly 80% of the current visible vertical dot capacity;
- fills every currently visible column;
- uses configured non-Drift category IDs as horizontal layers from bottom to top;
- leaves hidden canonical columns empty after the fixture reset;
- resets transient avalanche/queue counters and sets generated mass to the fixture mass;
- never touches authoritative sediment or persistence.

This is intentionally a fixture, not a physical rain process. Its purpose is to make later redistribution visually auditable.

## Literature direction for the curiosity fork

### BCRE

Bouchaud, Cates, Ravi Prakash and Edwards separate a pile into **immobile/static** and **rolling** populations. The moving population is transported down-slope and exchanges mass with the static bed by erosion/deposition. The key conceptual gain over a pure toppling automaton is that *being in motion is stateful*.

SEDIMENT-008 already approximates part of this using explicit rolling grains and erosion.

### Partial fluidization / Aranson–Tsimring

Aranson and Tsimring introduce an additional order parameter representing the transition between static and flowing granular states, coupled to the flow equations. A region can therefore become partially fluidized rather than every grain independently deciding to topple from a single local threshold.

For Strata the useful product idea is not to port a continuum PDE. It is to make a **bounded discrete order field** that can spread spatially while category-bearing grains remain discrete dots.

### Hysteresis

Granular flow has a robust start/stop asymmetry: static material needs a larger forcing to begin moving than already-flowing material needs to continue. This is the same qualitative reason Daerr–Douady-type avalanches can change from thin downhill tongues to large events with an uphill-moving failure front.

## `oslo-vessel-fluid`

New curiosity-only model:

```text
testingcheats model oslo-vessel-fluid
```

It uses frozen SEDIMENT-008 as its substrate and adds a transient integer fluidity field per canonical column:

```text
fluidity = 0      static
fluidity = 1..8   increasingly fluidized region
```

### Nucleation

The field does **not** activate from ordinary stable rain. It is nucleated only by the same severe Oslo failure that already creates a SEDIMENT-008 rolling seed:

```text
selected relief >= 4
→ normal front rolling seed
→ source + destination fluidity := 8
```

This retains the product invariant that ordinary quiescent Oslo evolution is exact until a structural/severe event happens.

### Propagation and hysteresis

Once fluidity exists:

- it diffuses only one neighbouring site per physics tick;
- already-fluidized material can persist while local downhill relief is only `>= 1`;
- severe relief can sustain a fluidized region;
- flat/unsupported activity decays;
- field strength is bounded `0..8`.

Thus static **start** remains relief 4 while dynamic **stop** is near relief 1.

### Static ↔ rolling exchange

A sufficiently fluidized site (`fluidity >= 4`) with positive downhill relief can release at most one settled grain per field tick into the existing physical rolling layer.

The order field itself carries no mass and no CategoryId. Every actual grain remains a dot with category identity. Mass can only move through the existing static columns / rolling-grain queues.

The model therefore explores:

```text
Oslo prepares metastable terrain
→ severe failure nucleates a fluidized patch
→ patch spreads as a region
→ region continuously recruits actual dots into the rolling layer
→ dots advect downhill / erode / remove support
→ field decays as slope and moving layer die
→ pile returns to static Oslo
```

This is deliberately **inspired by**, not numerically equivalent to, BCRE or Aranson–Tsimring.

## Diagnostics

`testingcheats status` for the curiosity model adds:

```text
fluid=active:<sites>
      activations:<total>
      releases:<total>
      peak:<sites>
      last=<activations>/<releases>
```

SEDIMENT-008 front counters remain visible as well.

## Machine gates

The local validator must prove:

1. frozen `oslo-vessel-front` existing focused tests remain green;
2. 2,000 ordinary quiescent drives of `oslo-vessel-fluid` match frozen front exactly with zero fluid activations/releases;
3. a severe failure nucleates the field immediately and it expands beyond the seed pair;
4. a deterministic wall fixture produces fluid releases, multiple simultaneously mobile grains, eventual quiescence, and exact mass conservation;
5. `testingcheats fill` parses and fills exactly 80% of current visible height with horizontal configured-category layers;
6. current VW/canonical boundaries remain respected;
7. debug stable-flow cadence is harness-only and does not modify sediment/persistence authority;
8. formatting, strict Clippy, all tests, and help smoke pass.

## Human test

### First: frozen front with stable cadence

Use:

```text
testingcheats model oslo-vessel-front
testingcheats fill
```

Shrink horizontally, then re-expand. Optionally keep `fallspeed 64x` or queue an `advance`.

Judge:

- avalanche shape/distribution should match the accepted SEDIMENT-008 character;
- flow speed should remain visually steady while the front is active;
- speed should no longer surge simply because the front becomes cheaper to compute;
- rolling avalanche and falling dots should feel like parts of the same visual clock.

### Then: curiosity model

Repeat with:

```text
testingcheats model oslo-vessel-fluid
testingcheats fill
```

Judge only qualitatively:

- does the failure read as a *region changing state* rather than a chain of isolated dot decisions?
- does the front spread spatially in a coherent way?
- does it retain the category-layer redistribution that made SEDIMENT-008 legible?
- does it stop naturally rather than fluidizing the whole vessel forever?
- is it more interesting than the frozen front, or merely more complicated?

No production promotion follows automatically from a positive curiosity result.
