---
id: UX-CONTRAST-001
kind: work
state: completed
authority: accepted
created: 2026-09-27
updated: 2026-09-27
summary: Unify contrast resolution for Strata-owned solid colored selection surfaces, including named terminal colors.
---

# UX-CONTRAST-001 — shared selection foreground contrast

## Owner decision

All existing Strata-owned solid colored selection/highlight surfaces use one contrast resolver. Named and truecolor RGB backgrounds share the existing color normalization and luma rule; terminal-owned Reset/indexed colors fall back to the active theme foreground. No Idle/category special case or automatic light/dark adaptation is introduced.

## Root cause

The view-local contrast helper recognized only `Color::Rgb` and chose white for every named variant. The shared appearance color normalizer already maps named variants, including `Color::White`, to their canonical RGB values.

## Result and regression evidence

The bounded fix replaces that duplicate logic with the appearance resolver and applies it to every existing colored selection surface: Layer modal, Balance summary and detail, Settings rows/overlays, and Command Palette. Tests cover white/black, the luminance threshold, every named-color/RGB pair, and Reset/indexed fallbacks. The built-in theme's named variants remain unchanged.

Formatting, strict Clippy, 519 unit tests, 24 integration tests, CLI help smoke, and diff hygiene pass. A disposable-profile TUI smoke opened Balance with Idle selected, observed black foreground on the white row, cycled Balance rows through dark/bright colors, and rendered Layer, Settings, and Command Palette selections. No render-unit harness was added; shared resolver tests plus PTY screen output and unchanged centralized call-site coverage provide the regression proof.
