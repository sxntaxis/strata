---
id: PERF-001
kind: work
state: complete
created: 2026-09-06
updated: 2026-09-07
authority: working
summary: Make the accepted Classic/Rain-001 sandbox materially faster without changing physics, then require the debug testing runtime to make an honest attempt at its requested 128x cadence instead of being capped by a fixed 4 ms frame budget.
---

# PERF-001 — Exact Classic high-speed runtime

## Owner gate

RAIN-001 passed machine validation. Human review found both uniform Classic and compensational Hybrid visually broad while falling, with no visible focus/nozzle, and preferred the Hybrid relief so far. The remaining blocker is observational speed: `testingcheats fallspeed 128x` feels only about 4x, making long-form morphology review impractical.

Do not retune rain or settlement in this unit. Preserve:

- `momentum-grounded-contact` Classic settlement;
- rugged/memory/slope/anchor/momentum semantics and RNG order;
- RAIN-001 75% full-width / 25% broad focus bias and compensational waypoint law;
- one ingress per simulated second;
- renderer output and theme/color semantics.

## Diagnosis

The debug sandbox had two independent performance ceilings.

First, Classic still paid dense work on every gravity sweep even after CLASSIC-012:

- visit every visible grid cell, including air;
- rebuild the grounded-column suffix index every sweep;
- clear an already-false `mobilized` matrix after every sweep/spawn;
- rescan the full grid to recover a grain count already represented exactly by `total_generated`.

Second, the app gave ordinary testing sand only 4 ms of CPU per render cycle regardless of requesting 16x, 64x, or 128x. At the normal render cadence this can cap an otherwise faster engine to a small fraction of one core, so the configured multiplier describes debt generation rather than achieved simulated time.

## Exact runtime optimization

The authoritative grid does not change.

Maintain an ephemeral runtime index for Classic:

- row occupancy bitsets, updated after every vertical/diagonal/bonus move and ingress placement;
- the exact bottom-connected grounded-column index from CLASSIC-012, now retained across sweeps and updated in place;
- invalidate/rebuild the mirrors after resize, clear, direct fill, or experiment-profile changes.

Gravity still processes rows bottom-to-top and preserves the existing parity-dependent left/right order. Sparse iteration visits exactly the occupied cells that the dense scan would encounter in that row. Grains only move into the already-processed row below, so no future source can enter the current row during that pass.

For consecutive grains that are provably immobile because the direct blocker is grounded and both ordinary diagonals are blocked, the dense law consumes exactly one physics RNG draw and performs no mutation. PERF-001 advances the same xorshift RNG by the exact number of such draws using a linear jump transform before the next potentially moving grain. No RNG draw is removed or reordered relative to an observable mutation.

Classic metadata sync becomes O(1): Classic never owns `mobilized` state and never deletes generated mass, therefore `surface.grain_count = total_generated` is exactly the previous `physical + pending` result. The physical grid remains available for explicit diagnostics/tests.

## Debug-build/runtime policy

`testingcheats` exists only in debug builds. Cargo's default unoptimized dev code generation therefore materially distorts the product-facing testing lab. Set `profile.dev.opt-level = 2` while retaining debug assertions.

Scale only the cooperative testing CPU budget:

- 1x–4x: 4 ms;
- 5x–16x: 8 ms;
- 17x–64x: 16 ms;
- 65x+: 32 ms;
- perceptual Oslo/flowviz retains its existing 12 ms presentation budget.

This does not change simulated cadence or physics. It only allows accelerated Classic/Hybrid debt to be consumed faster while leaving a bounded interval for input/render work each frame.

## Exactness gates

Optimized and dense-reference execution must match exactly for both Uniform Classic and RAIN-001 Hybrid across multiple seeds:

- complete grid/category topology;
- physics/rain/repose RNG states;
- local repose and memory fields;
- focus/waypoint/counter state and rain-region counters;
- pending drive order;
- frame count and sweep direction;
- generated/grain metadata;
- vertical/diagonal movement counters.

After an optimized interval, both engines must also remain identical through subsequent dense-reference execution.

## Performance target

Use a 190x48 terminal-cell Classic Hybrid probe with the accepted 1-second ingress and 32-ms physics clock. Run the optimized code under the **dev profile**, because that is what the owner runs with `cargo run`.

Report:

```text
PERF_001_HYBRID_RATE ... optimized_x=... reference_x=... speedup=...x
```

The headless dev-profile capacity gate is `optimized_x >= 160`, leaving enough solver headroom for the bounded 32 ms accelerated frame budget to plausibly sustain an interactive requested 128x once rendering/input are included. If it cannot reach 160x, stop and report the measured ceiling; do not change physics, lower gravity frequency, drop ingresses, or fake the multiplier. The owner remains authority on whether live `fallspeed 128x` is actually fast enough.

## Human gate

After native PASS:

```text
testingcheats clear
testingcheats model hybrid
testingcheats fallspeed 128x
```

The owner should be able to observe long-form RAIN-001 relief materially faster while short-term rain and resulting morphology remain the same model. If 128x still falls behind in practice, status/performance evidence should drive the next exact optimization rather than rain retuning.

## Native + human closure — 2026-09-07

PERF-001 passed exact optimized/reference validation and full repository checks. The native Hybrid probe reached a median 1691.83x headless capacity (samples 1691.83x, 1542.23x, 1700.34x), and the owner confirmed that interactive `testingcheats fallspeed 128x` now feels appropriately fast while preserving the accepted rain/Classic appearance. The resulting performance machinery is retained as the baseline for RAIN-002 and later Advance work.
