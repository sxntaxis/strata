---
id: SEDIMENT-015D
kind: work
state: active
created: 2026-09-04
updated: 2026-09-04
authority: working
summary: Replace SEDIMENT-015C's full-height uniqueness raster and ingress freeze with a spatially truthful O(N) mobile raster and live baseline rain, while retaining machine-green unit custody, observed event geometry, exact stack order, and frozen Oslo/front physics.
---

# SEDIMENT-015D — Truthful real-time raster

## Entry condition

SEDIMENT-015C-R1 is machine-green through 351 tests plus its explicit 5k/10k/20k/40k scaling probe, but the owner rejected the human gate.

Observed owner evidence:

```text
- playback effectively froze before the colored collapse could finish;
- CPU aggregate still looked mostly idle;
- red/yellow material appeared at implausible vertical positions;
- ordinary rain disappeared during the active avalanche;
- the unit-grain direction remained an improvement but was not acceptable.
```

The 015C human result is therefore:

```text
FAIL_TRUTHFUL_REALTIME_PRESENTATION
```

## Diagnosis

### 1. Full-height uniqueness search is not a valid renderer

015C rebuilt settled occupancy and, for each active carrier, searched potentially the entire viewport height for a unique free cell. The machine scaling evidence already exposed the consequence on the owner's native host:

```text
5k  raster ~0.66 s
10k raster ~1.50 s
20k raster ~3.46 s
40k raster ~7.18 s
```

That cost is principally serial/UI-thread work, so low aggregate CPU utilization does not imply headroom in the blocked render path.

### 2. Uniqueness was prioritized above spatial truth

A carrier whose causal cell was occupied could be drawn tens of rows away from its actual `x/y`. CategoryId accounting remained exact, but the picture could falsely place red/yellow units high on a lateral wall. A display-resolution collision is preferable to fabricating a remote location.

### 3. Micro separation duplicated work and could perturb the observed path

015C still ran four local-relaxation iterations before rasterization. 015D treats the already-observed rolling lane as presentation authority and removes this PBD-like relocation from the new profile.

### 4. Rain was deliberately disabled

015C zeroed the active-flow rain accumulator and did not advance falling ingress during perceptual flow frames. That removed the 015B waiting crust but also made the sky visibly stop. This is rejected product behavior.

## 015D contract

`oslo-vessel-front-grains` now selects a new truthful constructor layered strictly over 015C:

```text
unit carrier       = enabled
micro interpolation = enabled
observed geometry   = enabled
truthful raster     = enabled
```

The 015C constructor remains unchanged as a machine control.

### Spatial truth outranks per-frame uniqueness

For every on-screen carrier, the renderer starts from the carrier's causal rounded `x/y` and may use only a fixed 3x3 local stencil. X remains constrained to the real current source/destination edge. Y may differ by at most one dot.

If all local cells are occupied, the carrier shares the nearest truthful cell. It is not dropped from accounting and is never moved farther away to manufacture uniqueness.

This means:

```text
internal mass conservation: exact 1:1
rendered spatial claim: truthful within one dot
pixel uniqueness under dense occlusion: best effort, not authority
```

### O(N) raster path

015D uses one flat mobile occupancy array and a constant-size stencil per carrier. It does not build static-settled BTree occupancy and does not scan viewport height per carrier.

Expected asymptotic raster work is O(N) in active carrier count.

### Observed lanes, not presentation repulsion

For 015D, `relax_unit_micro_positions()` is a no-op. Existing `ideal_x/ideal_y` interpolation and observed segment geometry remain; the renderer no longer spends CPU moving grains away from those paths.

015B/015C retain their old micro-relaxation behavior as controls.

### Rain remains alive during flow

During 015D testing flow:

- baseline rain continues at one wall-clock grain per second;
- falling ingress advances at the ordinary effective grain-frame cadence;
- acceleration does not multiply rain rate;
- `commit_next_drive_if_quiescent()` remains outside the special flow frame, so rain cannot mutate the active pile until authoritative flow is quiescent;
- presentation-only catch-up also advances existing rain once per effective frame, not once per carrier substep.

Thus the sky remains alive without recreating 015B's accelerated waiting-rain crust.

## Frozen authority

015D must not change:

- Oslo thresholds `{1,2}`;
- front trigger, erosion, support-loss, coast/runout;
- physical rolling/site decisions;
- physical CategoryId custody;
- MotionId one-unit semantics;
- observed-edge-only routing;
- re-entry continuity;
- exact ordered settlement;
- persistence/schema;
- canonical production profile.

No production physics file is changed by this unit.

## Required native evidence

Before returning to the owner human gate:

1. 015D constructor differs from 015C only by the truthful presentation flag.
2. Truthful raster never places a carrier more than one dot from causal rounded y, and x remains on the current observed edge.
3. Dense overlap keeps all carriers in accounting and accepts local occlusion instead of remote relocation.
4. 015D micro relaxation is a no-op and cannot mutate carrier/physics state.
5. Active-flow rain advances visually while settled columns remain unchanged.
6. 64x active flow keeps the baseline one-grain-per-second rain clock rather than zeroing or accelerating it.
7. Frozen-front parity and exact final CategoryId stack remain green.
8. Full 015C, 015B, 015A, legacy conservative-flowviz, and Oslo regressions remain green.
9. Run explicit 5k/10k/20k/40k 015D scaling evidence and compare it with the certified 015C timings. The new raster must show the intended order-of-magnitude behavior; do not tune away unit carriers if it does not.
10. fmt, strict Clippy, full tests, help smoke, and command parser pass.

## Human question

Only after machine green:

> Does a large rainbow collapse now remain responsive enough to observe to completion, keep ordinary rain visibly alive, and place moving colors only where their causal unit paths plausibly are?
