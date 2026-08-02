---
id: INTERACTION-001
kind: work
state: completed
authority: accepted
created: 2026-08-02
updated: 2026-08-02
---

# INTERACTION-001 — explicit input and terminal authority

## Objective

Make every keystroke and process exit resolve through one visible, testable contract. Viewing, editing, commands, mandatory emergency behavior, and terminal restoration must not depend on accidental precedence or hidden fallbacks.

## Accepted invariants

- View mode and text-edit mode are explicit and visually distinguishable.
- Historical data changes only after one explicit commit.
- Cancel discards the complete draft.
- Command keys remain commands outside edit mode.
- All text characters, including ordinary command letters and Unicode, are available inside edit mode.
- Terminal restoration runs exactly once on normal quit, detach, runtime error, and panic.
- Runtime failure attempts explicit checkpoint recovery without hiding the original error.
- Configured, unbound, disabled, contextual, and mandatory actions are semantically distinct.
- No action is reachable through an undocumented fallback.
- The command atlas and command palette display runtime truth.
- Mandatory Quit remains under persistence-recovery custody.

## Certified sequence

### INTERACTION-001A — report description edit mode

Status: implemented and certified in PR #56.
Issue completed: #19.

- report-log view is read-only;
- Confirm enters edit mode for one stable session identity;
- the persisted description becomes a complete draft;
- command letters, spaces, and Unicode edit only the draft;
- Enter commits once; Esc discards the draft;
- persistence succeeds before memory changes;
- failed commit retains the draft and visible recovery state;
- deletion remains a separate command;
- the UI displays VIEW versus EDIT state;
- Quit from report context returns an application exit decision.

Accepted authority: STRATA-D034 through STRATA-D035.

### INTERACTION-001B — terminal lifecycle guard

Status: implemented and certified in PR #57.
Issue completed: #20.

- one RAII terminal session owns raw mode, alternate screen, cursor, flushing, and ratatui terminal state;
- explicit close, Drop, startup failure, and panic converge on one exactly-once restoration boundary;
- draw, poll, and read errors attempt one direct emergency checkpoint;
- primary runtime error kind and text remain authoritative;
- checkpoint and cleanup outcomes are attached only as context;
- panic restores terminal state without claiming persistence success;
- Linux PTY tests certify unchanged termios state and one restoration execution on quit, detach, draw/poll/read failure, and panic.

Accepted authority: STRATA-D036 through STRATA-D037.

### INTERACTION-001C — keymap truth

Status: implemented and certified in PR #58.
Issue completed: #24.

- Bound, Unbound, and Disabled are explicit distinct states;
- null physical-key entries remove only that key, while `unbind_actions` disables an action;
- contradictory bound-and-disabled configuration fails closed;
- Ctrl-C Quit is mandatory, separate, non-configurable, and recovery-safe;
- F1 is an ordinary configurable default;
- Confirm, Cancel, ReportToday, and report Detach behavior is declared as named contextual policy;
- one resolver owns direct, contextual, mandatory, and absent action outcomes;
- disabled actions are unreachable through direct keys, aliases, and the palette;
- atlas rows and control hints derive from runtime state;
- Backspace Disable and Delete Unbind remain visibly distinct.

Accepted authority: STRATA-D038 through STRATA-D040.

## Program certification

INTERACTION-001 passes:

- formatting;
- strict Clippy with all targets/features and warnings denied;
- 181 library tests;
- 9 CLI lifecycle tests;
- 6 configuration-authority tests;
- 1 report-help regression test;
- 12 SQLite/TUI process tests;
- 2 temporal-authority tests;
- 3 terminal-lifecycle PTY process tests.

## Closure

INTERACTION-001 is complete. Explicit editing, host-terminal custody, key state, contextual routing, mandatory emergency behavior, recovery-safe Quit, and command-atlas/runtime parity now share accepted authority. The next active frontier is the criterion-by-criterion reconciliation of partially satisfied issues #5, #10, and #13.