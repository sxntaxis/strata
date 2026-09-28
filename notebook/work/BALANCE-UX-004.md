---
id: BALANCE-UX-004
kind: work
state: candidate
authority: working
created: 2026-09-27
updated: 2026-09-27
summary: "Use one-cell ordinary-modal content inset with Balance-specific vertical rhythm, without turning Balance into a fixed-width content card."
---

# BALANCE-UX-004 — one-cell modal inset and vertical calm

## Owner direction

The BALANCE-UX-003 geometry is structurally accepted, but runtime review found modal content too close to its borders. An initial BALANCE-UX-004 candidate tried surplus-driven 1/2/4/6-cell horizontal inset. Owner review of the validated runtime rejected that behavior as excessive: the larger side padding was worse than allowing Balance names and right-aligned metrics to remain far apart on wide overlays.

The owner explicitly prefers the simpler rule: **one horizontal content cell per side at every ordinary modal size**, while retaining the calmer vertical grouping already introduced. Balance remains full-width inside that inset; it does not adopt a fixed narrow centered content column or a separate bounded information span.

## Accepted candidate

Modal geometry remains unchanged: content is the hard minimum, one third of the terminal is the comfort floor, and the configured frame margin clamps the result.

Inside that frame, ordinary modal content uses exactly **one horizontal cell of inset per side**. The inset does not grow with surplus width. Only a physically tiny inner area that cannot preserve even one content cell may collapse it. The modal geometry already budgets the content minimum plus this inset under normal conditions.

Layer/Strata uses exactly **one vertical content cell above and below** whenever the inner area has two spare rows; that inset collapses to zero only when the terminal cannot afford it. Its selected-row background therefore remains separated from the frame by one cell on all four sides at ordinary sizes.

Balance keeps the totals, meter, and category rows responsive across the full width remaining after the one-cell inset. A large name-to-metric gap on wide overlays is explicitly acceptable and preferred over larger side padding. Its vertical rhythm remains compact inside groups while preserving the existing breathing rows around the totals/meter/list groups; those rows collapse before content on constrained terminals.

## Local verification facts — 2026-09-27

- The superseded 1/2/4/6-cell candidate passed formatter, strict Clippy, all-feature tests (542 unit + 24 integration; 20 ignored), CLI help, isolated TUI checks at 200×60 / 80×24 / 40×14, and SQLite doctor.
- Owner runtime review rejected its horizontal inset growth after seeing the wide Layer result, then refined the retained vertical-calm behavior to exactly one top/bottom content cell for ordinary list modals.
- The one-cell correction passed formatter, strict Clippy, all-feature tests (543 unit + 24 integration; 20 ignored), and CLI help.
- An isolated debug TUI profile with the `testingcheats fill` fixture exercised Balance and Layer at 200×60, 80×24, and 40×14. Both retain one horizontal content cell per side at ordinary sizes. Layer uses exactly one top/bottom content cell when two spare rows exist, and collapses it at 40×14; Balance keeps its existing group rhythm. The command-palette route opened Layer at 40×14.
- The profile exited normally and passed `sqlite-doctor`. The wide Balance name-to-metric gap remains intentionally large under the corrected owner preference.
- SEDIMENT-016's separate exact persisted-mass/restart promotion proof remains open.

## Non-goals

This unit does not change:

- Balance arithmetic or meter normalization;
- report/category ordering;
- selected-layer frame color;
- historical preview behavior;
- sediment physics or resize semantics;
- custom-range interaction/keybindings;
- Settings or Command Palette geometry;
- theme/color authority.
