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

Status: active.
Issue: #19.

- add an explicit report-log edit state with stable session identity and draft text;
- use Confirm to enter editing from a selected log row;
- while editing, plain text and Unicode modify only the draft;
- Enter commits once; Esc cancels the complete draft;
- modified global emergency bindings remain deliberate;
- failed commit preserves the draft and visible recovery state;
- deletion remains a separate command;
- tests cover command precedence, Unicode, cancel, commit, and failed persistence.

### INTERACTION-001B — terminal lifecycle guard

Issue: #20.

- introduce one RAII owner for raw mode, alternate screen, and cursor restoration;
- separate terminal restoration from application finalization;
- preserve the original runtime error while attaching cleanup/recovery context;
- attempt an emergency checkpoint on draw, poll, or read failure;
- add panic restoration without claiming persistence success;
- certify PTY state after quit, detach, error, and panic.

### INTERACTION-001C — keymap truth

Issue: #24.

- preserve explicit Bound, Unbound, and Disabled action state;
- route F1 and contextual aliases through one declared policy;
- remove hidden Confirm, Cancel, ReportToday, and other fallback execution;
- declare any mandatory emergency binding separately and visibly;
- make atlas output reflect configured and mandatory truth;
- certify every current fallback and remapped alias.

## Current edge

Implement INTERACTION-001A only. Do not redesign keymap fallback semantics inside the edit-mode unit except where a modified emergency binding must remain deliberately reachable while editing.
