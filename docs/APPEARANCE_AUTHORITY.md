# Appearance authority

Status: accepted; APPEARANCE-001 implementation candidate
Last reviewed: 2026-09-06

## Purpose

Appearance is user-owned presentation. A theme supplies named colors and Strata-specific UI mappings; it does not own category identity, physics, chronology, or persistence authority. Theme changes may recolor presentation but must not mutate sand topology, RNG state, category IDs, sessions, or historical meaning.

## Theme contract

Themes are profile-local TOML files in `config/themes/*.toml`. The built-in theme is `default` and cannot be shadowed by a file with the same ID.

A theme has:

- `schema = 1`;
- `[theme]` metadata with a display `name` and `appearance = "dark" | "light" | "any"`;
- an arbitrary named `[palette]` of `#RRGGBB` swatches;
- optional `[sand].colors`, naming the subset eligible for category color cycling;
- optional `[ui]` mappings from Strata UI components to palette swatches or, where representable without color mixing, the sentinel `default`.

If `[sand]` is omitted, all palette swatches are eligible category colors. The palette has no fixed length and swatch names have no required color vocabulary. A theme may use names such as `mauve`, `ocean`, or `volcano`; Strata does not require semantic names such as red/green/blue.

`default` is reserved as presentation semantics, not a palette key. It means Strata leaves that color under external terminal/environment authority rather than claiming an RGB value.

The `idle` sand/UI role currently requires an explicit RGB swatch because idle grains participate in the same Braille color-reduction path as category grains. Background/foreground and non-sand UI roles may use `default`.

## Category color ownership

A category does not persist a theme slot number. It owns a theme-independent RGB color anchor. The active theme resolves that anchor to the nearest eligible sand swatch in perceptual OKLab space.

This makes theme changes portable across palettes of different lengths and naming conventions:

```text
category color anchor
        ↓
active theme eligible sand colors
        ↓
nearest perceptual swatch
        ↓
resolved display RGB
```

The existing SQLite `categories.color_index` physical column is retained for schema compatibility in APPEARANCE-001. Legacy non-negative indices still decode through the historical built-in palette. Newly synchronized non-idle categories encode the exact RGB anchor in a tagged positive integer carried by the same column. The column name is compatibility storage, not the new semantic model. Portable SQLite/CSV interchange preserves that raw value exactly.

CLI projection schema remains unchanged: its legacy `color_index` field is a compatibility projection to the nearest historical built-in color. It is not full-fidelity appearance authority; the SQLite bundle remains the full-fidelity interchange path.

## Category color UX

The existing Layers interaction remains authoritative:

- select a category;
- `Shift+←` / `Shift+→` changes its category color;
- the same controls select the color while forging a new layer.

Strata derives the cycling wheel from the active theme rather than trusting declaration order. Eligible swatches are converted to OKLCH and stably ordered by hue; near-neutral colors follow chromatic colors and use lightness ordering. Same-hue ties use lightness/chroma/key ordering.

The derived wheel is interaction only. Its numeric position is never persistent category identity.

## Theme selection

Settings owns global theme selection. Selecting a theme writes profile-local `appearance.toml`; category color anchors remain unchanged. The TUI resolves categories and UI roles against the newly selected theme immediately.

`--ignore-config` deliberately ignores both `appearance.toml` and profile-local custom themes and uses only the built-in default theme.

## Built-in baseline

The built-in `Strata Default` theme preserves the existing twelve historical category RGB values as eligible sand swatches, while the eligible set is no longer architecturally fixed at twelve.

Classic presentation defaults remain:

- physics: `momentum-grounded-contact`;
- Braille color reduction: `rgb-luma-safe`.

Theme selection must not change either physics or Braille dot geometry.

## Deferred work

APPEARANCE-001 does not yet implement terminal light/dark auto-detection or contrast adaptation policy. The accepted next layer is background-aware appearance with explicit `auto | dark | light` control and `strict | soft | safe` adaptation, with manual theme intent remaining higher authority than automatic correction.
