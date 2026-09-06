---
id: CLASSIC-013
kind: work
state: candidate
created: 2026-09-06
updated: 2026-09-06
authority: working
summary: Pause further Classic performance work after CLASSIC-012 and add two non-physics review tools: a read-only fill/fillhalf stratigraphy distribution report for the owner-observed cross-layer grains, plus selectable Braille foreground-color reduction profiles that preserve the existing RGB blend as the exact control.
---

# CLASSIC-013 — Stratigraphy + Braille color review

## Owner gate

CLASSIC-012 is machine-green. Its exact grounded-column cache preserved the accepted `momentum-grounded-contact` physics and improved the native dense-wall probe by a median 5.08x. Human review confirms the performance improvement is live, although the debug sandbox can still consume roughly 70% of one CPU thread.

The owner does **not** want another performance pass yet. Two visual questions remain open first:

1. during `fillhalf` relaxation, higher-band colors can appear surprisingly far down or laterally displaced (the earlier red-dot example); determine how much real category redistribution/mixing exists before deciding whether any physics change is warranted;
2. one Braille terminal cell represents up to eight independently categorized sand dots but can display only one foreground color; compare several reduction semantics to see whether layer boundaries/readability can improve.

`momentum-grounded-contact` remains the owner-selected Classic baseline. This unit must not alter its physics.

## A — read-only stratigraphy distribution diagnostic

`testingcheats fill` / `testingcheats fillhalf` now retain a **debug-only descriptor** of the initial rainbow footprint:

- horizontal fill span;
- initial vertical span;
- category order and exact initial vertical band per Fixture category;
- exact initial grain count per band.

No per-grain identity, velocity, provenance state, or persistent schema is added.

The command:

```text
testingcheats classic stratigraphy
```

is valid only for a Classic/Hybrid testing sandbox after `fill` or `fillhalf`. It writes a read-only report to:

```text
$XDG_CACHE_HOME/strata/classic-stratigraphy.txt
```

(or `~/.cache/strata/classic-stratigraphy.txt`).

The report measures the **current distribution**, not an invented historical path:

- current tracked mass per initial Fixture band;
- grains currently above/below their band's original vertical range;
- maximum above/below displacement relative to that band;
- grains outside the original fill span and maximum lateral escape;
- currently unsupported tracked grains;
- bottom-to-top category-rank drops as a bounded stratigraphic-order signal;
- number of visible Braille cells containing tracked material and how many contain multiple Fixture categories;
- top positional outliers with coordinate/category/band/span/support facts.

The descriptor is invalidated by `clear` or `resize`, because those operations destroy the exact fill-relative coordinate reference. The report explicitly states that it does **not** reconstruct an individual grain's path. If that later becomes necessary, it requires a separately authorized trace/provenance experiment.

The diagnostic must not consume RNG, mutate the grid, change repose/memory, or affect movement counters.

## B — Braille color-blend profiles

The production renderer's pre-existing color law remains the exact control:

```text
rgb
```

`SandEngine::render()` still calls that control. The Classic testing sandbox can switch render-only profiles with:

```text
testingcheats classic colorblend rgb
testingcheats classic colorblend linear
testingcheats classic colorblend oklab
testingcheats classic colorblend dominant
testingcheats classic colorblend dominant-soft
```

Query the active profile with:

```text
testingcheats classic colorblend
```

Profiles:

- `rgb` — exact legacy weighted arithmetic mean in encoded sRGB; baseline/control.
- `linear` — weighted RGB mixture after sRGB linearization, then re-encode to sRGB; expected to make mixed cells brighter than encoded-space averaging.
- `oklab` — weighted mean in OKLab after sRGB→linear conversion, then convert back; intended as the perceptual-hue comparison.
- `dominant` — use the category with the largest dot count; exact ties choose the lower `CategoryId` deterministically. This maximizes layer crispness but can hide minority dots chromatically.
- `dominant-soft` — retain every category but square each category's dot count before the existing encoded-RGB mean. Exact ties remain exact ties; a majority gains progressively stronger influence without a hand-tuned fixed blend coefficient.

All five profiles preserve the Braille dot mask/character exactly. Only the one foreground RGB assigned to that glyph may differ. A single-category Braille cell must retain that category's exact RGB in every profile.

## Hard semantic boundary

Do not change:

- `momentum-grounded-contact` ordinary-contact law;
- grounded-column cache behavior;
- repose, memory, slope bias, anchors, momentum eligibility, RNG consumption, sweep order, movement count, or mass;
- fill/fillhalf geometry or Fixture category order/colors;
- Oslo/H4 physics or renderer selection;
- production/persisted schema, snapshots, category semantics, or provenance;
- application scheduling, frame cadence, or performance policy.

The new color profiles are debug/testing presentation experiments only. No profile becomes the production default from this unit.

## Required machine proof

1. `SandEngine::render(categories)` is exactly equal to `render_with_color_blend(..., rgb)`.
2. Single-category Braille cells are byte/foreground-equivalent across every blend profile.
3. Mixed-cell tests prove `dominant`/`dominant-soft` alter only foreground color, not the Braille character, and `linear` remains distinct from legacy encoded-sRGB averaging.
4. Switching Classic colorblend profiles leaves grid, physics/repose RNG state, local repose, repose memory, and movement counters unchanged.
5. A fresh fill/fillhalf stratigraphy report is read-only; initial unrelaxed fill reports zero stratigraphic rank drops and no grains outside the original fill span.
6. A synthetic cross-band/lateral outlier is surfaced by the report without mutating physics state.
7. `clear`/`resize` invalidate the fill-relative report reference.
8. CLASSIC-012 exact-cache equivalence, CLASSIC-011 grounded-contact gates, all prior Classic metrics, fillhalf, Oslo, provenance, category idempotence, parser/help, full tests, fmt, and Clippy remain green.

## Human comparison

Physics control:

```text
testingcheats classic experiment momentum-grounded-contact
testingcheats fillhalf
```

While/after the interesting mixing state is visible:

```text
testingcheats classic stratigraphy
```

Then keep the **same live grid** and switch only rendering:

```text
testingcheats classic colorblend rgb
testingcheats classic colorblend linear
testingcheats classic colorblend oklab
testingcheats classic colorblend dominant
testingcheats classic colorblend dominant-soft
```

Do **not** refill between colorblend switches when comparing them. The owner should judge the exact same physical arrangement under five color reductions.
