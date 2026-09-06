---
id: APPEARANCE-001
status: authored-candidate
owner_gate: native-validation-then-human
updated: 2026-09-06
---

# APPEARANCE-001 — Theme contract and category color anchors

## Owner decisions

- Preserve the current Layer UX: `Shift+←` / `Shift+→` owns category color cycling.
- Do not freeze the historical twelve-color array as a theme contract.
- Theme palettes use arbitrary author-defined swatch names and arbitrary length.
- Optional `[sand].colors` filters the palette for category selection; omitted means all swatches.
- Strata, not theme declaration order, derives a perceptual OKLCH hue wheel for cycling.
- Do not invent color-role translation such as `red = maroon`.
- `[ui]` maps actual Strata presentation components directly to palette swatches.
- Use `default`, not `terminal`, for values left under external presentation authority.
- Category identity is independent of theme. Theme switching maps the category's persisted color intent perceptually.
- `momentum-grounded-contact` remains the Classic physics baseline.
- `rgb-luma-safe` becomes the Classic color-blend baseline.
- Automatic terminal light/dark detection and strict/soft/safe adaptation are deferred until the manual theme contract is certified.

## Authored implementation

- Added `src/appearance.rs` and an embedded `themes/strata-default.toml`.
- Added profile-local `appearance.toml` theme selection and `config/themes/*.toml` discovery.
- Added Settings → Appearance → Theme selection.
- Preserved Layer color cycling, now against the active theme's derived sand wheel.
- Added theme-aware category/UI rendering paths.
- Retained SQLite schema v1. Legacy category color indices decode unchanged; new category colors persist exact RGB anchors through a tagged positive value in the existing compatibility column.
- Kept CLI schema stable by projecting anchors back to the nearest legacy color index; portable SQLite interchange remains full-fidelity.
- Promoted Classic default blend to `rgb-luma-safe` without changing physics.

## Required proof

Native validation must prove parser/schema behavior, arbitrary palette length, hue ordering, legacy category compatibility, exact anchor round-trip, theme-switch non-mutation of physics/history, Layer cycling persistence, Settings persistence, `--ignore-config`, Classic baseline preservation, full repository checks, and existing performance/physics gates.
