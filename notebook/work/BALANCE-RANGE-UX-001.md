---
id: BALANCE-RANGE-UX-001
kind: work
state: candidate
authority: working
created: 2026-09-27
updated: 2026-09-27
summary: "Replace the modifier probe with explicit bracket-selected Balance range boundaries while preserving inclusive ReportWindow authority."
---

# BALANCE-RANGE-UX-001 — explicit range boundaries

## Owner decision

The physical modifier/Caps Lock probe is no longer a product gate. Terminal character bindings may collapse physical Shift and Caps Lock into the same case-bearing character event; Strata accepts that ordinary terminal limitation instead of adding Caps-Lock inversion, terminal-specific keyboard modes, tmux assumptions, or physical-key detection.

Custom Balance range editing therefore uses explicit punctuation handles rather than modifier-letter gestures:

- `[` selects the start boundary;
- `]` selects the end boundary;
- Left/Right moves the selected boundary by exactly one operational day and applies the result live;
- Esc clears the selected handle before ordinary Balance back/close behavior resumes;
- selecting the opposite bracket switches handles;
- without a selected handle, Left/Right retains whole-interval navigation;
- `r` remains the direct typed range editor for distant jumps.

The bracket actions are configurable Balance actions, not hardcoded input bypasses. They follow the existing Bound / Unbound / Disabled authority and are available through Settings and the command palette.

## Boundary presentation

Balance chrome always shows the two interval boundaries with a spaced en dash. The domain `ReportWindow` remains inclusive; only its TUI projection uses an exclusive visible end:

- internal `2026-09-21..2026-09-21` -> `Sep 21 – Sep 22`;
- internal `2026-09-21..2026-09-27` -> `Sep 21 – Sep 28`;
- internal `2026-09-01..2026-09-30` -> `Sep 1 – Oct 1`.

An active handle is bracketed in the footer, e.g. `[Sep 21] – Sep 28` or `Sep 21 – [Sep 28]`. The existing outer whole-period chevrons remain. No Day/Week/Month/Range label or additional key-hint chrome is added.

Current partial periods remain partial. Their visible end is exactly one day after the last internally included operational day, not a synthesized future calendar boundary.

## Domain preservation

`ReportWindow.start` and `ReportWindow.end` remain the first and last included operational-day keys. Report filtering, live-session slicing, persistence, historical-sediment selection, and whole-window navigation keep the existing inclusive semantics.

The typed `r` editor now presents the same UI boundary convention:

- From = inclusive internal start;
- To = exclusive visible end;
- `From < To` is mandatory;
- commit converts `To` back to inclusive internal end by subtracting one day.

No SQLite/schema migration or second interval type is introduced.

## Boundary movement

Start selected:

- Left expands the start one day earlier;
- Right contracts the start one day later;
- start may never move after the internal end.

End selected:

- Left contracts the internal end one day earlier;
- Right expands the internal end one day later;
- internal end may never move before start or later than the current operational day.

A one-day range cannot contract further. Failed movements are no-ops and keep the selected handle. Selecting a handle alone does not convert a preset into custom mode; the first successful movement does.

Preset changes, period cycling, entering detail, starting another Balance editor, Settings/palette takeover, and closing Balance clear the transient handle.

## Superseded probe

The earlier standalone Crossterm probe established only synthetic tmux observations. Those observations are no longer required for product acceptance and the probe executable is removed from the candidate. No physical keyboard or Caps Lock capture is required.

## Validation target

The implementation must preserve the existing Balance instrument/layout, historical preview animation, selected-layer frame color, one-cell modal inset, report arithmetic, storage authority, and SEDIMENT-016 behavior. Focused tests cover boundary formatting/conversion, movement limits, typed-editor conversion, configurable bracket bindings, and existing whole-window navigation regressions.

Local native Rust/TUI validation remains required for the authored candidate.

## Non-goals

This unit does not change:

- inclusive `ReportWindow` domain semantics;
- CLI `report --from/--to` inclusive semantics;
- SQLite schema or persistence;
- sediment runtime or accepted sediment doctrine;
- historical activity correction semantics;
- Balance arithmetic or meter normalization;
- modal spacing or theme/color authority.
