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

- Layer Tag/name input owns printable characters before configurable command routing. Uppercase letters and symbols are text rather than hidden Shift-letter commands.
- `Ctrl+e` without Shift renames the selected non-Idle Layer. Rename is an isolated editor: printable text and Backspace/Delete edit the name draft, Enter validates/saves and returns, and Esc discards and returns. `Ctrl+Shift+E` is a distinct unbound chord.
- Remove the durable layer-description editor and palette-search surface; preserve existing description values only for SQLite/portable-interchange compatibility and round-trip. Dropping the stored field/data requires a separate versioned schema/interchange migration.
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
- Layer modal width may expand for the visible Tag/name editor within existing terminal clamps.

## Implementation candidate

- Default keymap now binds Main sand clearing to Backspace/Delete, layer rename to unshifted `Ctrl+e`, Layer reorder to `Ctrl+Up/Down`, and existing-Layer recolor to `Ctrl+Left/Right`; `r` and `l` remain actions but are unbound by default.
- Layer modal text ownership runs before ordinary configured action routing for unmodified printable characters and Backspace/Delete.
- Rename uses a distinct layer-name draft and does not alter the active session Tag; SQLite category sync preserves stable CategoryId and history.
- Shift-arrow period cycling is removed. Month-scale boundary adjustment uses one shared civil-month helper with end-of-month clamping.
- Ledger Date acceleration uses the same civil-month helper; Time acceleration remains timestamp-safe sixty-minute arithmetic.
- Multi-tag filter focus is now an explicit transient selector rather than an immediate facet toggle/focus hybrid.
- Balance selection stores the selected `CategoryId` and derives its current row index at render/action time.
- Layer Detail width now derives from untruncated Tag/date/time/effect columns without the previous hard cap; spare modal width distributes across stable columns, Tag remains left-aligned, Date/Time are visibly separated, and Effect remains right-aligned. Cross-day rows use separate compact Date and Time cells.

## Owner review correction — layer rename and ledger geometry

The owner clarified that the Ctrl+e editor should change the Layer's displayed name, not its Tag or durable description; category descriptions are unnecessary as a user-facing feature. The owner specified unshifted Ctrl+e, distinct from Ctrl+Shift+E. Existing stored category-description values are preserved for compatibility/round-trip; the editor and palette-search surface are retired.

The owner then reported two Layer Detail visual defects from populated-profile screenshots: a selected multi-tag value was ellipsized despite the modal expansion, and a cross-day row placed dates and times together in a different middle cell than neighboring rows. The requested presentation is a compact date span in Date (for example `Sep 10-11`) and only the start/end times in Time (for example `23:50-06:00`). The repair sizes columns from their actual content, retains the ordinary separate columns for cross-day rows, and still respects terminal bounds.

The owner's follow-up screenshot review found the revised row cells were only sized to their minimum text widths, leaving a large blank region to the right and bunching Date/Time/Effect together. The clarified requirement is to distribute spare modal width across stable columns, keep Tag at the left and Effect aligned to the right, and visibly separate Date and Time. Rename follow-up: Backspace removes one character per keypress, no underline on the draft, and no `Rename layer · Enter save · Esc cancel` title.

## Validation status

**VALIDATION UPDATE — NATIVE-GREEN / OWNER INTERACTION REVIEW NEXT.** Rust 1.98.1 validation passed `cargo fmt --all -- --check`, strict Clippy with all targets/features and warnings denied, all-feature tests (585 library + 24 integration/process tests; 20 ignored), and CLI help. The first Clippy pass identified now-unused `report_period_prev` / `report_period_next` helpers after Shift-arrow period cycling was removed; the dead helpers were deleted, and the checks then passed. Formatting cleanup was applied.

A loaded test-profile PTY confirmed `Ctrl+E` opens the isolated metadata editor, printable draft text is accepted, Esc cancels/returns, and Balance opens. The multi-tag selector's displayed row/facet was not conclusively confirmed in PTY capture, so owner review should inspect multi-tag `f` selection, Left/Right facet movement, Enter/`f` apply, Esc cancel, and inert vertical movement. The isolated test profile is `/mnt/Tokyo/Lab/.tmp/opencode/strata-balance-tag-filter-ux-001-test-profile`; SQLite doctor passes after the smoke. No production profile was used. Native process regression also verifies Backspace clears only Idle sand and preserves other category mass across restart.

At bundle authorship, the source environment lacked `cargo`, `rustc`, and `rustfmt`, so the candidate made no native-green claim. The validation update above records the subsequent native checks and the remaining owner interaction review.

### Latest integrated correction

Rust 1.98.1 passed the declared formatter check, strict Clippy, all-feature tests (588 library + 24 integration/process tests; 20 ignored), and CLI help after the owner clarified rename semantics and supplied the Layer Detail screenshots. On the loaded behavior-coherence test profile, one Backspace changed `Lab` to `La`; typing `b` and committing restored `Lab` with the same CategoryId. The rename title hint was absent. SQLite doctor passed after the profile smokes; the profile is test-only.

Unit coverage verifies Ctrl+e without Shift maps to rename while Ctrl+Shift+E is unbound; rename trims valid names and rejects empty, reserved Idle, and duplicate names; and cross-day labels remain `Sep 10-11` with an independent `23:50–06:00` time span. A loaded-profile PTY showed the cross-day row with separate, distributed columns and the full multi-tag text while its second facet was selected. Responsive sizing distributes spare width across content-derived columns and terminal-clamps narrow layouts. The user's tag-filter test profile had a live TUI and was not used. Owner final visual verification remains welcome.

## Owner final correction — STRATA-D075

The owner accepted a final interaction/presentation correction after reviewing the local-agent screenshots:

- active-Layer Tag changes remain live previews, but opening Layer snapshots the original Tag; Enter accepts the preview and Esc restores that opening Tag. Structural Layer operations remain immediate and are not rolled back;
- `+` / `=` and `-` / `_` retain polarity semantics only before Tag text editing begins. Once typing/backspace/tag cycling establishes text editing, those symbols are text. A Tag may contain them but cannot be manually started with them;
- empty Tags render as neutral `—`, including untagged filter presentation;
- the contextual destructive key is `Ctrl+x`: Layer archives the selected Layer; Balance summary requests permanent Layer deletion with Enter/Esc confirmation; Layer Detail deletes only the selected persisted ledger row and ignores the Add row;
- permanent Balance Layer deletion removes sessions and tag history but does not rewrite sediment. SQLite keeps a hidden non-restorable identity/color tombstone solely so already-materialized sand retains valid CategoryId/color meaning and the identifier cannot be reused;
- Balance summary omits exact-zero Layer rows for the selected period while retaining stable-ID selection;
- `[` / `]` share one boundary-edit transaction. Left/Right previews ±1 day, Shift+Left/Right previews ±1 civil month, Enter accepts the full edited range, and Esc restores the complete opening range;
- Layer Detail returns to the original full-row proportional geometry: 1:1:1 for Tag/Time/Effect and 1:1:1:1 when Date is present, including cross-day rows. The modal—not individual columns—widens until every visible value fits its assigned equal cell; truncation is allowed only when the actual terminal width prevents further growth.

The implementation candidate is based on owner-corrections head `1ce73f3688556e918fe359f1b9707d94f4d0928e`. This authorship environment has no Rust toolchain, so D075 carries no formatter/Clippy/test claim yet; native validation and owner screenshot testing are the next gate. Settings/palette behavior remains explicitly deferred.
