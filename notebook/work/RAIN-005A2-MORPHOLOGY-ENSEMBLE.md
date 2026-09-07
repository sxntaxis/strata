---
id: RAIN-005A2
kind: work
state: candidate
created: 2026-09-07
updated: 2026-09-07
authority: working
summary: Characterize RAIN-004 as a statistical family across arbitrary viewport geometries and seeds, rather than optimizing against one owner window or one lucky/unlucky run; no runtime morphology changes.
---

# RAIN-005A2 — Morphology ensemble baseline

## Why this pass exists

RAIN-005A remained runtime-exact to RAIN-004, yet the owner's next representative run looked visually more homogeneous than the earlier strongest RAIN-004 captures. That is expected evidence of run-to-run stochastic variation, not evidence that the measurement-only diagnostics changed rain.

The owner explicitly rejects optimizing RAIN-005B for one screen/window. Strata must behave across fullscreen, split panes, portrait-ish terminals, arbitrary aspect ratios, font zooms and resize histories.

RAIN-004 therefore remains the **best-so-far reference family**, not a perfect target and not one canonical screenshot. Future morphology must target equal-or-greater heterogeneity over the family while preserving broad/nozzle-free falling rain.

## Machine strategy

Do not enumerate common display resolutions. Use deterministic low-discrepancy sampling over a continuous width/height domain.

### Morphology ensemble

An ignored native probe samples 16 width/height cases across terminal geometry and assigns a distinct deterministic seed to each case.

Each case runs optimized exact RAIN-004 Hybrid with five category phases. A category phase lasts one **derived one-dot-excess mound timescale** for that viewport/kernel, rather than an arbitrary raw number of grains or seconds.

For every case report independent morphology primitives:

- mean category thickness CV;
- mean adjacent thickness-profile correlation;
- mean adjacent normalized-profile total variation;
- mean adjacent centroid shift;
- total centroid span across the category stack;
- derived mound scale in equivalent layers.

Then report p10/p50/p90 across the ensemble. These distributions become the RAIN-004 baseline envelope for later non-inferiority comparison. They are not yet acceptance thresholds.

### Analytical arbitrary-geometry/nozzle ensemble

A second ignored probe uses 64 low-discrepancy samples across:

- dot width;
- dot height;
- normalized focus position;
- open-sky fraction.

It analytically evaluates the exact RAIN-004 two-candidate broad focus kernel and reports distributions of:

- expected canonical-1x airborne population;
- KL bits per ingress;
- total visible nozzle information;
- normalized RMS kernel width;
- mound scale in equivalent layers.

This is deliberately geometry-normalized and does not depend on the owner's current terminal.

## Owner capture evidence

The owner-supplied RAIN-005A report from one representative run measured:

```text
width_dots=378
height_dots=192
visible_airborne_now=2
geometry_expected_airborne_1x=1.587302
nozzle_information_bits_now=0.071229907
nozzle_information_bits_geometry_1x=0.056531672
focus_kernel_rms_width_fraction=0.221793820
mound_equivalent_layers=3.927160
mean adjacent thickness-profile correlation=0.456496070
mean adjacent profile total variation=0.082208075
mean adjacent centroid shift=0.029749457
```

The owner judged this particular morphology run less interesting/more thickness-homogeneous than the strongest RAIN-004 captures. These values are one observation, not a target.

## Hard boundary

RAIN-005A2 adds test/diagnostic evidence only.

It must not change:

- RAIN-004 bias;
- correlated meander;
- category rephase;
- terrain/edge steering;
- ingress distribution;
- Classic settlement;
- RNG ordering in runtime;
- renderer;
- persistence;
- Advance/catch-up;
- performance implementation.

## Next decision

Only after the ensemble is native-run should RAIN-005B be designed. Candidate laws should be expressed in normalized/equivalent-layer/kernel units and tested against the RAIN-004 ensemble, not against one viewport or screenshot.
