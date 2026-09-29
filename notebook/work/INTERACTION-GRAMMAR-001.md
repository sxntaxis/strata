---
id: INTERACTION-GRAMMAR-001
kind: work
state: candidate
authority: working
created: 2026-09-28
updated: 2026-09-28
summary: "Unify Strata keyboard ownership and modifier meaning across Main, Layer, Balance boundaries, ledger editing, and multi-tag filtering."
---

# INTERACTION-GRAMMAR-001 — coherent keyboard grammar

## Exact base

This pass starts from the owner-supplied, natively validated BALANCE-BEHAVIOR-COHERENCE-001 bundle at head `0dc12cf476316486852475c0999f38d40d5fe681` (uploaded bundle SHA256 `583fa044da433ff1036047e062e4d29651f0118214c41a484835df9144be4dea`). STRATA-D071/D072/D073 remain historical authority; STRATA-D074 supersedes only the keyboard details explicitly listed below.

## Owner decision — STRATA-D074

- Layer Tag/name/metadata input owns printable characters before configurable command routing. Uppercase letters and symbols are text rather than hidden Shift-letter commands.
- `Ctrl+E` replaces `Shift+E` for durable Layer metadata. Metadata is an isolated editor: printable text and Backspace/Delete edit, Enter saves and returns, Esc discards and returns, and Layer Tag/navigation/color/reorder actions are inert while it owns input.
- Main `Backspace` clears only Idle sand; Main `Delete` clears all sand. The old `c` / `Shift+C` defaults are removed. Editors continue to own Backspace/Delete locally.
- Plain arrows operate the focused value or selection. Shift is reserved for a larger form of that same directional adjustment. Ctrl owns structural/alternate Layer operations.
- Existing Layer: Up/Down selects, Left/Right cycles Tag, `Ctrl+Up/Down` reorders, `Ctrl+Left/Right` changes color. The previous Shift-arrow reorder/color bindings are retired.
- New Layer: plain Left/Right changes color because there is no Tag axis yet.
- Balance no longer uses Shift+Left/Right to cycle Day/Week/Month/Range. `t`, `w`, and `m` remain explicit presets.
- `balance_range` loses default `r`; `[` / `]` are the normal keyboard route to custom boundaries. The underlying range action remains unbound/configurable/palette-reachable until the deferred Settings/palette pass.
- `balance_log_activity` loses default `l`; the navigable `+ Add entry…` row plus Enter is the canonical add route. The underlying action likewise remains unbound/configurable/palette-reachable for the deferred Settings/palette pass.
- An active Balance boundary moves one operational day with Left/Right and one civil month with Shift+Left/Right, preserving the day where possible and clamping at month end plus the existing range safety limits.
- Ledger Date uses one day / one civil month for plain/Shift horizontal adjustment. Ledger Time uses one minute / one hour. Tag has no Shift accelerator.
- Multi-tag filtering: a one-tag row toggles immediately with `f`; on a multi-tag row the first `f` enters an in-row selector without changing filter state, Left/Right chooses a facet, Enter or `f` applies and exits, Esc cancels, and vertical or Shift-arrow movement is inert while selecting.
- Balance summary selection follows stable Layer identity rather than a dynamically sorted row index.
- Layer modal width may expand for the visible Tag/name/metadata within existing terminal clamps.

## Implementation candidate

- Default keymap now binds Main sand clearing to Backspace/Delete, metadata to `Ctrl+E`, Layer reorder to `Ctrl+Up/Down`, and existing-Layer recolor to `Ctrl+Left/Right`; `r` and `l` remain actions but are unbound by default.
- Layer modal text ownership runs before ordinary configured action routing for unmodified printable characters and Backspace/Delete.
- Metadata preserves the pre-edit Tag draft while the durable description is edited, then restores that draft after save/cancel.
- Shift-arrow period cycling is removed. Month-scale boundary adjustment uses one shared civil-month helper with end-of-month clamping.
- Ledger Date acceleration uses the same civil-month helper; Time acceleration remains timestamp-safe sixty-minute arithmetic.
- Multi-tag filter focus is now an explicit transient selector rather than an immediate facet toggle/focus hybrid.
- Balance selection stores the selected `CategoryId` and derives its current row index at render/action time.
- Layer modal preferred width incorporates the active text/completion width as well as layer names.

## Validation status

**VALIDATION UPDATE — NATIVE-GREEN / OWNER INTERACTION REVIEW NEXT.** Rust 1.98.1 validation passed `cargo fmt --all -- --check`, strict Clippy with all targets/features and warnings denied, all-feature tests (585 library + 24 integration/process tests; 20 ignored), and CLI help. The first Clippy pass identified now-unused `report_period_prev` / `report_period_next` helpers after Shift-arrow period cycling was removed; the dead helpers were deleted, and the checks then passed. Formatting cleanup was applied.

A loaded test-profile PTY confirmed `Ctrl+E` opens the isolated metadata editor, printable draft text is accepted, Esc cancels/returns, and Balance opens. The multi-tag selector's displayed row/facet was not conclusively confirmed in PTY capture, so owner review should inspect multi-tag `f` selection, Left/Right facet movement, Enter/`f` apply, Esc cancel, and inert vertical movement. The isolated test profile is `/mnt/Tokyo/Lab/.tmp/opencode/strata-balance-tag-filter-ux-001-test-profile`; SQLite doctor passes after the smoke. No production profile was used. Native process regression also verifies Backspace clears only Idle sand and preserves other category mass across restart.

At bundle authorship, the source environment lacked `cargo`, `rustc`, and `rustfmt`, so the candidate made no native-green claim. The validation update above records the subsequent native checks and the remaining owner interaction review.
