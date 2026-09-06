---
id: CLASSIC-015
kind: work
state: candidate
created: 2026-09-06
updated: 2026-09-06
authority: working
summary: Promote owner-selected `rgb-additive` as the Classic/Hybrid Braille color baseline and add render-only luminance/neutral/contrast comparison profiles plus an explicit dark/light/neutral background preview policy, with zero physics, RNG, stratigraphy-distribution, or persistence changes.
---

# CLASSIC-015 — Luminance + terminal-background color review

## Owner gate

Human review selected CLASSIC-014 `rgb-additive` over legacy `rgb`; it is now the Classic/Hybrid default color reduction. The owner wants to explore whether complementary cancellation should preserve source lightness instead of always moving toward white, while avoiding foreground colors that disappear on either dark or light terminals.

The physical baseline remains `momentum-grounded-contact`. Stratigraphic redistribution remains accepted unchanged. This unit is presentation-only.

## Color profiles

Keep all previous profiles and add:

```text
testingcheats classic colorblend rgb-luma
testingcheats classic colorblend rgb-luma-safe
testingcheats classic colorblend rgb-mid
testingcheats classic colorblend rgb-contrast
```

All four begin from the exact legacy encoded-RGB weighted mean and use the same cancellation detector/`cancellation^2` response as `rgb-additive`. If there is no chromatic cancellation, output remains exact legacy RGB. Single-category cells remain exact source RGB.

### `rgb-luma`

Strong cancellation converges toward an achromatic OKLab neutral whose `L` is the weighted mean OKLab lightness of the source colors. This is the raw source-lightness reference and may approach light/dark extremes.

### `rgb-luma-safe`

Same as `rgb-luma`, but clamp the neutral target to:

```text
OKLab L in [0.42, 0.72]
```

This intentionally prevents a fully cancelled occupied glyph from approaching either black or white.

### `rgb-mid`

Strong cancellation converges toward a fixed neutral:

```text
OKLab L = 0.57
```

This is the theme-independent control: stable and intentionally insensitive to source lightness.

### `rgb-contrast`

Uses source lightness plus an independently selectable background preview policy. All targets remain inside the same middle-lightness philosophy; no profile is allowed to choose literal black/white as a contrast endpoint.

```text
background=neutral -> target L = 0.57
background=dark    -> clamp source to [0.42,0.72], then at least 0.64
background=light   -> clamp source to [0.42,0.72], then at most 0.50
```

The purpose is to test whether a background-aware neutral is materially better than a universal safe neutral.

## Background preview policy

Add a separate render-only control:

```text
testingcheats classic colorbackground
testingcheats classic colorbackground neutral
testingcheats classic colorbackground dark
testingcheats classic colorbackground light
```

Default is `neutral`.

For CLASSIC-015 this policy affects only `rgb-contrast`. It does not recolor `rgb-additive`, `rgb-luma`, `rgb-luma-safe`, `rgb-mid`, or legacy profiles.

Do **not** add OSC 11/background probing in this unit. Automatic terminal detection would introduce terminal request/response lifecycle behavior and should be considered only after the owner selects a background-aware color semantic. If `rgb-contrast` wins, a later presentation/settings unit may promote `dark|light|auto` into a real user setting with `auto -> safe neutral fallback` when probing is unavailable.

## Baseline promotion

For new Classic/Hybrid testing sandboxes:

```text
colorblend = rgb-additive
colorbackground = neutral
```

`SandEngine::render()` remains the generic legacy `rgb` renderer used outside the Classic/Hybrid debug sandbox; this unit does not silently alter unrelated H4/Oslo presentation.

## Hard boundary

Do not change:

- Classic movement, grounding, repose, momentum, sweep order, mass, or RNG consumption;
- grounded-index/cache behavior or CLASSIC-012 performance semantics;
- fill/fillhalf geometry, Fixture category identity/order/colors, or physical stratigraphy distribution;
- Oslo/H4 physics or rendering selection;
- persistence, schema, terminal lifecycle, terminal input parsing, or configuration files.

Changing a color profile or background preview must be non-retroactive and render-only.

## Required proof

1. New Classic/Hybrid sandbox defaults to `rgb-additive` + `neutral` background preview.
2. `rgb-additive` remains byte-equivalent to CLASSIC-014 semantics.
3. All new profiles preserve single-category cells exactly and preserve Braille glyph geometry.
4. Non-canceling mixtures remain exact legacy RGB under every new profile/background preview.
5. Exact Red/Cyan cancellation demonstrates distinct raw-luma, safe-luma, fixed-mid, dark-contrast, neutral-contrast, and light-contrast results.
6. `rgb-luma-safe` and all `rgb-contrast` policies stay away from both black and white.
7. `dark` contrast produces a lighter neutral than `neutral`; `light` produces a darker neutral than `neutral` on the same complementary composition.
8. Profile/background switching leaves grid, mass, RNG states, repose state, movement counters, and experiment selection unchanged.
9. CLASSIC-011 physics metrics and CLASSIC-012 performance equivalence remain unchanged.
10. Parser/help/full tests/fmt/Clippy remain green.

## Human comparison

Create one physical state and never refill while switching color modes:

```text
testingcheats fillhalf

# accepted baseline
testingcheats classic colorblend rgb-additive

# source-lightness variants
testingcheats classic colorblend rgb-luma
testingcheats classic colorblend rgb-luma-safe
testingcheats classic colorblend rgb-mid

# background-aware variant
testingcheats classic colorblend rgb-contrast
testingcheats classic colorbackground neutral
testingcheats classic colorbackground dark
testingcheats classic colorbackground light
```

Judge complementary Red/Cyan and Yellow/Blue mixed Braille cells, ordinary neighboring-color boundaries, perceived brightness continuity, and whether any occupied glyph becomes visually weak against the simulated dark/light background assumptions.
