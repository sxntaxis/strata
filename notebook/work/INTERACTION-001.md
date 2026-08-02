---
id: INTERACTION-001
kind: work
state: active
authority: working
created: 2026-08-02
updated: 2026-08-02
---

# INTERACTION-001 — explicit input and terminal authority

## Objective

Make every keystroke and process exit resolve through one visible, testable contract. Viewing, editing, commands, mandatory emergency behavior, and terminal restoration must not depend on accidental precedence or hidden fallbacks.

## Required invariants

- View mode and text-edit mode are explicit and visually distinguishable.
- Historical data changes only after one explicit commit.
- Cancel discards the complete draft.
- Command keys remain commands outside edit mode.
- All text characters, including ordinary command letters and Unicode, are available inside edit mode.
- Terminal restoration runs exactly once on normal quit, detach, runtime error, and panic.
- Runtime failure attempts explicit checkpoint recovery without hiding the original error.
- Configured, unbound, disabled, contextual, and mandatory actions are semantically distinct.
- No action is reachable through an undocumented fallback.
- The command atlas displays runtime truth.

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

Status: next.
Issue: #24.

- preserve explicit Bound, Unbound, and Disabled action state;
- route F1 and contextual aliases through one declared policy;
- remove hidden Confirm, Cancel, ReportToday, and other fallback execution;
- declare any mandatory emergency binding separately and visibly;
- make atlas output reflect configured and mandatory truth;
- certify every current fallback and remapped alias.

## Current edge

Implement INTERACTION-001C. Editing and host-terminal custody are now explicit. The remaining interaction risk is keymap truth: runtime behavior, configuration, contextual aliases, mandatory emergency controls, and the command atlas must describe the same reachable actions without hidden defaults.
