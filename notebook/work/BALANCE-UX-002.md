---
id: BALANCE-UX-002
kind: work
state: candidate
authority: working
created: 2026-09-27
updated: 2026-09-27
---

# BALANCE-UX-002 — adaptive summary geometry

## Runtime review

Owner review of BALANCE-UX-001 at both a wide terminal and a roughly half-width pane exposed three presentation defects:

- the default Balance overlay inherited a one-third terminal ratio, producing excess vertical emptiness on large terminals and squeezing the 45-cell instrument until its three totals touched on narrower panes;
- historical snapshot provenance and the centered period competed for the same bottom border and became visually tangled;
- the equilibrium `┼` used full foreground while the surrounding axis was dim status, making the center marker flash white instead of belonging to the line.

These are presentation defects only. Report arithmetic, interval semantics, history correction, snapshot authority, category ordering, keybindings, and sand physics remain unchanged.

## Candidate correction

Introduce one generic Ratatui-centered content rectangle helper. It receives desired **inner** width/height and clamps the resulting bordered overlay to the existing frame margin. Balance is the first consumer; other overlays are intentionally not migrated in this slice.

The default Balance summary derives its desired width from the larger of its row content and the 45-cell instrument plus side breathing room. Desired height derives from instrument chrome plus the actual category-row count, with one additional interior row when historical provenance must be shown. This makes the overlay content-driven rather than proportional to terminal area while preserving safe narrow-terminal clamping.

The instrument continues to use independently aligned Ratatui rects. At normal sizes the summary now guarantees enough room for the preferred 45-cell meter, avoiding the nine-character totals touching each other in 27-cell fallback geometry.

Historical snapshot provenance remains explicit, as required by sediment authority, but moves from a competing bottom-border title into one subdued interior status row. The bottom border is reserved for the centered `< period >` navigation object.

The equilibrium `┼` uses the same dim `theme_status` style as the inactive axis. The moving dot and active displacement remain polarity-colored, so equilibrium is structurally visible without becoming a white visual hotspot.

## Shared-layout boundary

The new centered-content helper is deliberately generic so Layer, Settings, command palette, and future overlays can migrate to one content-driven sizing authority after separate runtime review. This slice does **not** silently change their established geometry.

## Validation state

Source-side diff and bundle construction can be verified in the handoff environment, but Rust toolchain and TUI runtime validation are required on the local Strata machine before this candidate can be certified or merged.
