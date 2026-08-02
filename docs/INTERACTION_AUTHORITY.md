# Interaction authority

Status: partially implemented and certified
Program: INTERACTION-001
Current completed unit: INTERACTION-001A
Issues completed: #19
Last reviewed: 2026-08-02

## Purpose

Interaction authority determines whether an input is navigation, a command, text, confirmation, cancellation, or emergency control. The same physical key must not change historical data merely because focus or routing precedence is ambiguous.

## View and edit ownership

Report-log view mode is read-only. Navigation, report commands, deletion, modal cancellation, and quit remain commands.

Historical description editing begins only after an explicit Confirm action on a selected persisted session row. Edit state owns:

- the stable session ID;
- a complete draft copied from the current persisted description;
- visible edit-mode status;
- commit or cancellation responsibility.

Selection movement or row ordering does not change the draft's owning identity.

## Edit-mode input

While report description edit mode is active:

- unmodified character input, including ordinary command letters, spaces, and Unicode, appends to the draft;
- Backspace and Delete remove one Unicode scalar from the draft;
- Enter requests one commit;
- Esc cancels the complete draft;
- unrecognized modified input is ignored;
- a modified key executes only when the configured keymap resolves it to the deliberate emergency Quit action.

No plain character is interpreted as a global or report command while editing. Outside edit mode, command routing remains unchanged.

## Persistence boundary

A draft changes canonical history only after explicit commit.

SQLite authority performs one fenced session-description update. Memory changes only after that transaction succeeds.

Legacy-file authority edits a cloned session collection, writes it through the existing persistence boundary, and replaces in-memory state only after the write succeeds.

A failed persistence attempt:

- does not alter canonical in-memory history;
- retains the complete draft and stable session ID;
- enters the existing visible persistence-recovery state;
- allows retry or authority reload under the established failure contract.

A successful commit closes edit mode. Cancel closes edit mode without a persistence write.

Description edits do not invalidate daily sediment contributions because description text is not sediment-relevant chronology.

## Visible state

The report modal displays whether it is in:

- `VIEW · Enter edit · Esc back`; or
- `EDIT DESCRIPTION · Enter commit · Esc cancel`.

The selected row renders the live draft with an explicit cursor marker. Closing the report modal, changing UI mode, or resetting report state discards any uncommitted draft.

## Command separation

Deletion remains a separate configured command. Enter no longer exits the report-log view; it enters editing. Quit from report context returns an application exit decision instead of being silently ignored.

INTERACTION-001A does not redefine hidden keymap fallbacks or mandatory actions globally. Those semantics remain INTERACTION-001C.

## Certification

The exact implementation passed:

- formatting;
- strict Clippy with all targets/features and warnings denied;
- 167 unit tests;
- 9 CLI lifecycle tests;
- 6 configuration-authority tests;
- 1 report-help regression test;
- 12 SQLite/TUI process tests;
- 2 temporal-authority tests;
- doc tests.

Focused proofs cover command-letter text input, Unicode and spaces, Enter/Esc intent, modified emergency Quit isolation, failed-commit draft retention, and successful-commit closure.

## Remaining interaction authority

- INTERACTION-001B / issue #20 — process-wide terminal lifecycle restoration and emergency runtime failure custody.
- INTERACTION-001C / issue #24 — explicit Bound, Unbound, Disabled, and mandatory keymap semantics with atlas/runtime parity.
