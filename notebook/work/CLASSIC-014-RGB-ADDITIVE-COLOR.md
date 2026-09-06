---
id: CLASSIC-014
kind: work
state: candidate
created: 2026-09-06
updated: 2026-09-06
authority: working
summary: Keep the owner-selected legacy `rgb` Braille color reduction unchanged as the control and add one render-only `rgb-additive` comparison that lifts strong chromatic cancellation toward white instead of dull gray, with zero physics or stratigraphy changes.
---

# CLASSIC-014 — RGB additive cancellation review

## Owner gate

CLASSIC-013 is machine-green. Human review selected `rgb` as the best-looking Braille foreground reduction. The stratigraphy diagnostic is considered sufficient for now; no physical redistribution rule should change in this unit.

One renderer question remains: encoded-RGB averaging can turn strongly opposed colors into dull gray. With the testing palette, exact 50/50 Red+Cyan and Yellow+Blue mixtures collapse to neutral gray even though an emitted-light reading would trend bright/white.

The owner wants only this color experiment now. Classic physics, stratigraphy, performance work, fill/fillhalf, and category identity remain untouched.

## New profile

Add:

```text
testingcheats classic colorblend rgb-additive
```

`rgb` remains the exact legacy/default control.

`rgb-additive` starts from the exact same encoded-sRGB weighted mean as `rgb`, including its existing f32 weighting/truncation. It then measures how much source chroma disappeared in the mixture:

```text
source_chroma = weighted mean(max(R,G,B) - min(R,G,B)) of source colors
mixed_chroma  = chroma of the legacy rgb result
cancellation  = clamp(1 - mixed_chroma/source_chroma, 0, 1)
```

Neutral source colors (`source_chroma == 0`) remain exact legacy RGB.

The cancellation response is deliberately conservative:

```text
lift = cancellation²
output = legacy_rgb + (white - legacy_rgb) * lift
```

Consequences:

- no chromatic cancellation -> byte-identical legacy RGB;
- mild/unequal cancellation -> only a small lift;
- strong complementary cancellation -> progressively brighter/paler;
- complete chromatic cancellation -> white rather than gray;
- single-category cells remain exact category RGB;
- no dominant-color tie or hue flip is introduced.

This is a presentation experiment, not a claim that terminal Braille cells physically model pigment or light mixing. It is specifically an anti-mud reduction for the terminal's one-foreground-color limitation.

## Required examples

For exact 8-dot compositions:

```text
4 Red + 4 Cyan
rgb          -> (127,127,127)
rgb-additive -> (255,255,255)

4 Red + 4 Yellow
rgb-additive == rgb
```

Unequal Red+Cyan compositions must transition continuously rather than flipping to a dominant source color.

## Hard semantic boundary

Do not change:

- `momentum-grounded-contact` or any Classic movement/grounding/repose/momentum law;
- grounded-column cache or CLASSIC-012 performance semantics;
- fill/fillhalf geometry, Fixture colors/order, category IDs, or stratigraphy reporting;
- RNG state or consumption;
- sweep order, movement counters, mass, persistence, schema, provenance, Oslo/H4 behavior;
- `SandEngine::render()` production/control selection: it remains `rgb` unless the owner explicitly promotes another profile after human comparison.

## Required proof

1. `rgb` remains exact legacy behavior.
2. `rgb-additive` preserves every single-category cell exactly.
3. Exact 50/50 Red+Cyan cancellation becomes white while the Braille glyph geometry is unchanged.
4. A non-canceling Red+Yellow mix is exactly equal to `rgb`.
5. Unequal Red+Cyan mixtures lift continuously without a dominant-color discontinuity.
6. Profile selection is render-only and leaves grid, physics/repose RNG state, local repose, memory, counters, and mass unchanged.
7. CLASSIC-011/012 physics/performance exactness, CLASSIC-013 stratigraphy, parser/help, full tests, fmt, and Clippy remain green.

## Human comparison

Use one live physical state and do not refill between switches:

```text
testingcheats classic experiment momentum-grounded-contact
testingcheats fillhalf

testingcheats classic colorblend rgb
testingcheats classic colorblend rgb-additive
```

Judge mainly mixed boundaries involving Red/Cyan and Yellow/Blue. The desired result is removal of dull neutral-gray cancellation without making ordinary neighboring-color boundaries unnecessarily pale or glowy.
