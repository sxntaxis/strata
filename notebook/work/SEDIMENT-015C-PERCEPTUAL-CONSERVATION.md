---
id: SEDIMENT-015C
kind: work
state: active
created: 2026-09-04
updated: 2026-09-04
authority: working
summary: Strengthen the machine-green unit-carrier renderer so visible transport conserves not only CategoryId mass and causal x-edges but also observed rolling geometry, dense on-screen carrier visibility, and wall-clock progression under accelerated testing, without changing frozen Oslo/front physics.
---

# SEDIMENT-015C — Perceptual conservation

## Entry condition

SEDIMENT-015A-R2 is machine-green and established one unit carrier per active CategoryId mass unit, observed-edge-only routing, exact re-entry MotionId continuity, exact final stack order, exact-once discharge, and frozen-front parity.

SEDIMENT-015B is also machine-green, but the owner human gate failed. The failure is presentation fidelity, not conservation accounting.

Observed human findings from the rainbow-collapse gate:

```text
- a horizontal line of white waiting ingress appeared above the active pile;
- visible traffic still did not feel commensurate with later sediment redistribution;
- colored progression remained spatially strange despite one-unit carriers;
- playback became slower while CPU remained substantially idle;
- the result was directionally better than aggregate parcels but not acceptable.
```

The 015B human result is therefore:

```text
FAIL_PERCEPTUAL_CONSERVATION
```

## Diagnosis

Four mechanisms remain weaker than the owner-visible contract.

### 1. Waiting-rain crust

The testingcheats scheduler continued spawning one ordinary Drift grain per wall-clock second during an avalanche. Because authoritative drive settlement is quiescence-gated, those white grains could reach the pile surface and wait there as a horizontal line. That is neither avalanche mass nor a valid final state.

### 2. Bounded raster omission

015B searched only a small vertical neighborhood around each carrier. In dense transport, a valid unit carrier could therefore remain in accounting while no unique raster cell was found for it. Mathematical mass remained exact, but visible mass could be temporarily under-represented.

### 3. Y geometry reconstructed from later shadow state

015A/015B recorded the real horizontal lattice edge, but target elevation was commonly derived when the segment began replaying. By then the shadow/bed could have changed. This preserved causal CategoryId/x movement while allowing visually implausible vertical band reconstruction.

### 4. Fixed historical 64 ms wall-clock bottleneck

During visible flow the testingcheats scheduler ignored almost all of `fallspeed 64x`: one effective Oslo frame was admitted per 64 ms wall-clock interval and excess delay was discarded. Unit segments need several presentation steps, so backlog could grow while CPU remained idle.

## Contract

SEDIMENT-015C strengthens presentation without changing physics authority.

### Observed geometry, never predicted geometry

Each unit segment may now carry:

```text
observed_source_y
observed_target_y
```

These values are captured only when the corresponding physical unit event already exists.

For a direct settled transfer, target y is the authoritative destination stack depth at the event.

For rolling entry/hop, target y is the authoritative rolling-layer lane at the destination at the event, using settled height plus the already-present rolling depth. Consecutive rolling segments use the previous observed endpoint as the next observed source.

No segment may contain a future edge or future destination chosen by presentation.

### 015B remains a control

`new_front_unit_micro_flowviz_vessel` remains the 015B control constructor.

A new 015C constructor enables:

```text
flowviz_unit = true
flowviz_unit_micro = true
flowviz_unit_perceptual = true
```

The debug command:

```text
testingcheats model oslo-vessel-front-grains
```

selects the 015C constructor. Existing 015A/015B tests continue to exercise their earlier constructors.

### Dense raster visibility

For the 015C mode, raster placement searches the complete local vertical capacity of the authoritative source/destination edge rather than only `y +/- 4`.

Priority remains:

1. unique mobile cell;
2. no overlap with settled/falling display if capacity exists;
3. static overlap only as a final presentation fallback;
4. never collapse two mobile carriers into one cell.

Offscreen carriers remain clipped rather than pulled into view.

### Offscreen geometry stays offscreen

015C micro relaxation no longer clamps active unit y coordinates to the viewport. Negative or beyond-bottom observed coordinates remain distinct presentation geometry and are clipped only at render time. This prevents multiple over-height carriers from being projected onto the top row.

### Testingcheats ingress isolation

While the 015C unit renderer is in explicit/visible/presentation flow:

- do not spawn new baseline wall-clock rain;
- do not advance or launch waiting ingress during the special avalanche frame;
- existing ingress remains frozen at its current sky position until flow quiesces;
- authoritative avalanche physics continues.

`FallingDrive.y` is presentation/drive custody only until commit, and commit was already forbidden during explicit flow. This change is testing presentation behavior, not production Oslo semantics.

### Spend acceleration on the active renderer

For 015C only, wall-clock flow debt uses a bounded presentation multiplier derived from the requested fallspeed:

```text
1x   -> 1x flow clock
4x   -> 2x
16x  -> 4x
64x  -> 8x
128x -> 12x
```

The scheduler subtracts each 64 ms effective frame from the accumulator instead of zeroing accumulated debt. It runs multiple authoritative flow frames within the existing cooperative CPU deadline when due.

After each physical flow frame, presentation receives additional catch-up substeps based on unit backlog:

```text
<= 2,048  debt units -> 2 presentation substeps
<= 8,192             -> 3
>  8,192             -> 4
```

When physics is already quiescent, the same presentation-only substeps drain carriers without advancing ingress, fluidity, thresholds, RNG, or settled mass.

This is time compression and CPU expenditure. It never coalesces or drops mass.

## Physics authority boundary

015C must not alter:

- Oslo thresholds `{1,2}`;
- front start/erosion/support-loss/coast rules;
- downhill decisions or RNG;
- CategoryId physical custody;
- rolling/settled/discharge decisions;
- persistence/schema;
- canonical production profile.

The only production-solver-file hook change is that an already-created rolling MotionId appends its presentation segment through the geometry-recording helper. The helper cannot influence the rolling decision.

## Required native gates

Before another owner visual gate, require all of the following.

1. 015C constructor differs from 015B only by the perceptual presentation flag.
2. Observed segment y survives later shadow changes unchanged.
3. Two simultaneous rolling entries into one destination receive distinct observed rolling lanes.
4. A rolling second hop preserves the previous observed target as its next observed source.
5. Dense raster fixture places every on-screen unit carrier into a unique cell when local edge capacity exists beyond the old `+/-4` search.
6. Perceptual resize preserves bottom-relative observed geometry across wide/narrow/round-trip projection, including offscreen y.
7. Perceptual testing flow freezes waiting FallingDrive presentation.
8. 015C ordinary-drive physics matches frozen front exactly and drains to exact stack equality with zero unit misses.
9. Existing 015B semantic suite remains green unchanged.
10. Existing 015A unit suite remains green.
11. Legacy conservative-flowviz and Oslo suites remain green.
12. App testing clock proves 64x perceptual flow consumes accelerated presentation debt while wall-clock rain debt remains zero.
13. Run the explicit ignored 5k/10k/20k/40k native scaling probe and report `relax_us` and `raster_us` for each tier. These measurements are evidence, not hard cross-machine acceptance thresholds.
14. `cargo fmt`, strict Clippy, full tests, and help smoke pass.

## Human gate

Only after machine green:

```text
testingcheats model oslo-vessel-front-grains
testingcheats fill
testingcheats fallspeed 64x
```

Judge one or two broad collapses plus wide -> narrow -> hold -> wide.

Acceptance questions:

- Is the prior white waiting-rain line absent?
- Does each dense visible stream remain populated rather than thinning invisibly and later reappearing?
- Do color bands visibly peel and move along the evolving slope instead of reconstructing from later pile geometry?
- Does 64x remain visually trackable but materially faster, with no obvious long idle stalls while CPU is available?
- Does arriving visible mass now plausibly explain the sediment that appears?

Do not tune Oslo slope, front thresholds, rain focus, or avalanche frequency from this gate.
