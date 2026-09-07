---
id: RAIN-001
kind: work
state: superseded
created: 2026-09-06
updated: 2026-09-07
authority: working
summary: Superseded after machine-green and partial human acceptance: broad 75/25 rain passed the anti-nozzle gate and produced attractive relief, but the continuous cross-corridor focus walk read as one dominant lump and occupied-target nearest-free ingress looked artificial near a full top row.
---

# RAIN-001 — Compensational focus rain

## Owner direction

Classic settlement is now the accepted physics reference: `momentum-grounded-contact` with rugged local heterogeneity. The remaining physics-facing design question is ingress. The owner wants layers that remain visually continuous but vary strongly in lateral thickness and locus from one stratum to the next, closer to natural stratigraphic stacking than repeated parallel lasagna. At the same time, airborne rain must continue to read as broad homogeneous rain rather than a visible moving nozzle.

A previously human-liked Strata/Oslo rain screenshot came from the older broad-focus family that used 75% uniform full-width rain plus 25% weak focus-biased rain, with a slow persistent focus constrained away from the true side walls. That is retained as useful project evidence rather than inventing a new narrow emitter.

## Research synthesis

The useful geological analogue is **compensational stacking**, not literal erosion or Grand Canyon formation replication. Depositional systems commonly persist in one locus long enough to build relief and later shift so deposition statistically occupies lower accommodation elsewhere. Hybrid stochastic/compensational models are more appropriate inspiration than either perfect global low-seeking or completely memoryless random stacking.

Relevant literature used for design:

- Straub, Paola, Mohrig, Wolinsky & George, *Compensational stacking of channelized sedimentary deposits*, Journal of Sedimentary Research 79 (2009): intermediate stochastic compensation rather than perfect global compensation.
- Jerolmack & Paola, *Complexity in a cellular model of river avulsion*, Geomorphology 91 (2007): simple local/topographic rules can create persistent loci, avulsion and reoccupation without a globally prescriptive solver.
- Hajek & Straub, *Autogenic sedimentation in clastic stratigraphy*, Annual Review / related compensation literature: depositional relief and migration can generate lateral stacking heterogeneity without requiring every event to seek the global minimum.

Grand Canyon diagrams are visual inspiration only. Unconformities, erosion, lithologic changes and tectonic tilting are outside RAIN-001; rain must not fabricate those processes.

## Candidate semantics

`testingcheats model classic` remains the uniform-rain control.

`testingcheats model hybrid` becomes the single RAIN-001 candidate. Classic gravity/repose/momentum are unchanged; only target sampling/focus waypoint choice differs.

### Broad ingress

For every ingress target in `hybrid`:

- 75%: uniform over the entire current visible physical width;
- 25%: weak focus-biased target sampled broadly inside the focus corridor by choosing the nearer of two random corridor candidates;
- the biased stream never means `x = focus`, a Gaussian nozzle, or a fixed-radius spray;
- focus-only padding remains approximately 11.8% per side using the existing recursive-golden construction;
- ordinary full-width rain may and must continue to target both focus-padding regions.

### Persistent focus walk

Retain the accepted slow walk:

- focus lives only inside the golden focus corridor;
- a waypoint lies in the opposite outer sixth of the corridor, maintaining large persistent relocations rather than Brownian ±1 jitter;
- theoretical edge-to-edge focus traversal remains approximately 43,200 ingresses (~12 simulated hours at one ingress per second);
- focus advances by ingress count, not render cadence.

### Weak compensational waypoint choice

When a new opposite-side waypoint is required:

1. draw two stochastic waypoint candidates inside the same opposite outer-sixth band;
2. for each candidate, measure bottom-connected terrain height over a broad local window spanning roughly one sixth of the focus corridor;
3. choose the candidate with the lower mean broad terrain height;
4. exact equal broad relief remains random.

This is intentionally a **two-candidate tournament**, not a scan for the global minimum. Terrain is consulted only when choosing a new long-lived waypoint, never on each grain or each focus step. The smoothing window intentionally ignores Classic grain-scale ruggedness.

## Hard boundaries

RAIN-001 must not change:

- `momentum-grounded-contact` physics;
- rugged/repose/memory/anchor/momentum constants;
- gravity scan order or RNG semantics outside rain selection;
- category transport or mass conservation;
- live cadence (~one ingress per simulated second);
- full-width physical basin / closed visible side walls;
- resize semantics;
- persistence/schema/SandState;
- Appearance/theme behavior;
- production catch-up;
- ADVANCE experiments.

The compensational terrain scan occurs only at rare waypoint selection, so it must not become a meaningful live CPU hotspot.

## Machine evidence

Required focused gates:

1. bias split constant is exactly one focused ingress in four;
2. focus always remains inside the existing recursive-golden corridor;
3. both focus-padding regions receive ordinary rain over a deterministic long target sample;
4. short-window target distribution remains broad: every coarse horizontal bin receives rain and no bin dominates at nozzle-like share;
5. unequal broad terrain candidates deterministically choose the lower candidate without consuming tie RNG;
6. exact equal broad terrain retains stochastic choice;
7. uniform and hybrid continue to share byte-identical gravity behavior when ingress is removed from the comparison;
8. Classic baseline regression/metrics remain unchanged;
9. Appearance, parser, fill/fillhalf and repository gates remain green.

Capture the deterministic `RAIN_001_METRICS` line from the broad-rain test.

## Human gate

Compare only two modes; do not create a profile ladder:

```text
testingcheats model classic
testingcheats fallspeed 1x
```

then:

```text
testingcheats model hybrid
testingcheats fallspeed 1x
```

For morphology, the owner may accelerate `hybrid` or switch active categories over time to expose stacked color bands, but no new settlement profile is part of this pass.

Judge:

- airborne dots still look spatially homogeneous over short viewing intervals;
- there is no visible moving nozzle/column;
- long-lived favored locus migrates meaningfully rather than parking at a wall;
- successive color strata acquire visibly different thickness envelopes/shoulders/lateral centroids;
- strata remain broadly continuous rather than fragmenting into isolated blobs;
- aggregate pile has no persistent left/right lean;
- Classic avalanche character remains unchanged.

If the 75/25 candidate is clearly too strong or too weak, only the bias fraction may be reconsidered next. Do not reopen Classic settlement physics.

## Supersession — RAIN-002

RAIN-001 passed machine validation and its 75/25 broad-rain envelope passed the owner's short-term airborne anti-nozzle gate. Human high-speed review after PERF-001 preferred the Hybrid relief but found the long continuous focus walk too dominated by one large lump and found top-row nearest-free relocation visually artificial once ingress became crowded. RAIN-002 retains 75/25 and the accepted Classic settlement law while replacing only focus temporal structure and ingress-site selection.
