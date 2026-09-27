---
id: BALANCE-UX-004
kind: work
state: candidate
authority: working
created: 2026-09-27
updated: 2026-09-27
summary: "Add responsive interior modal spacing without turning Balance into a fixed-width content card."
---

# BALANCE-UX-004 — responsive modal inset and vertical calm

## Owner direction

The BALANCE-UX-003 geometry is structurally accepted, but runtime review found two remaining presentation problems:

- Layer/Strata and Balance content sit too close to their modal borders;
- Balance category names and right-aligned metrics become visually too disconnected as surplus modal width grows.

The owner explicitly rejected replacing the responsive layout with a fixed narrow centered content column. The selected direction is a dynamic version of the existing full-width Balance layout, combined with the calmer vertical grouping previously explored.

## Accepted candidate

Modal geometry remains unchanged: content is the hard minimum, one third of the terminal is the comfort floor, and the configured frame margin clamps the result.

Inside that frame, content receives **surplus-driven stepped inset**:

```text
constrained  -> 0 when even one cell would squeeze required content
minimum room -> 1 cell per side
more room    -> 2 cells per side
wide         -> 4 cells per side
roomy        -> 6 cells per side
```

The implementation derives the largest permitted step from width available beyond the content minimum. It is therefore responsive without percentage padding and collapses before actual content does.

Layer/Strata uses the same horizontal inset and may consume spare height as one or two rows of top/bottom padding. Its selected-row background is contained inside the inset rather than touching the frame.

Balance keeps the totals, meter, and category rows responsive across the available inset width; it does **not** adopt a fixed-width centered information card. Its vertical rhythm remains compact inside groups while explicitly preserving breathing rows around groups: one row before the instrument, one between meter and category list, and one reserved below the list when height permits. Those rows collapse before content on constrained terminals.

## Local verification facts — 2026-09-27

- Formatter, strict Clippy, all-feature tests (542 unit + 24 integration; 20 ignored), and CLI help smoke pass.
- An isolated debug TUI profile with the `testingcheats fill` fixture was exercised at 200×60, 80×24, and 40×14. Balance and Layer stayed centered and readable; roomy panes showed the responsive inset, while the constrained pane collapsed it and retained visible rows.
- The command palette remained reachable at 40×14 and opened Layer; the TUI exited normally and the profile passed `sqlite-doctor`.
- `docs/INTERACTION_AUTHORITY.md` records the accepted visual contract. SEDIMENT-016's separate exact persisted-mass/restart promotion proof remains open.

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
