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
- Configured, disabled, and mandatory actions are semantically distinct.
- No action is reachable through an undocumented fallback.
- The command atlas displays runtime truth.

## Certified sequence

### INTERACTION-001A — report description edit mode

Status: implemented and certified in PR #56.
Issue completed: #19.

- report-log view is read-only;
- Confirm enters edit mode for one stable session identity;
- the persisted description is copied into a complete draft;
- unmodified command letters, spaces, and Unicode edit only the draft;
- Enter requests one persistence commit;
- Esc discards the complete draft;
- only a configured modified Quit remains deliberate emergency behavior;
- SQLite and legacy-file memory change only after persistence succeeds;
- failed commit retains the draft and visible recovery state;
- deletion remains a separate command;
- report UI displays VIEW versus EDIT state and a draft cursor;
- modal close/reset discards uncommitted drafts;
- Quit from report context returns an application exit decision.

Accepted authority is recorded in `docs/INTERACTION_AUTHORITY.md` and STRATA-D034 through STRATA-D035.

### INTERACTION-001B — terminal lifecycle guard

Status: next.
Issue: #20.

- introduce one RAII owner for raw mode, alternate screen, mouse capture, and cursor restoration;
- separate terminal restoration from application finalization;
- preserve the original runtime error while attaching cleanup/recovery context;
- attempt an emergency checkpoint on draw, poll, or read failure;
- add panic restoration without claiming persistence success;
- certify PTY state after quit, detach, runtime error, and panic.

### INTERACTION-001C — keymap truth

Issue: #24.

- preserve explicit Bound, Unbound, and Disabled action state;
- route F1 and contextual aliases through one declared policy;
- remove hidden Confirm, Cancel, ReportToday, and other fallback execution;
- declare any mandatory emergency binding separately and visibly;
- make atlas output reflect configured and mandatory truth;
- certify every current fallback and remapped alias.

## Current edge

Implement INTERACTION-001B. Explicit text editing is now authoritative; the next risk is process-wide terminal custody. Every successful, failed, or panicking TUI path must restore the host terminal exactly once while preserving the original application error and recovery evidence.
