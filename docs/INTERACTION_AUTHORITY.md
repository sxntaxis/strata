# Interaction authority

Status: partially implemented and certified
Program: INTERACTION-001
Current completed unit: INTERACTION-001B
Issues completed: #19, #20
Last reviewed: 2026-08-02

## Purpose

Interaction authority determines whether an input is navigation, a command, text, confirmation, cancellation, or emergency control, and who owns the host terminal while the TUI is active. Ambiguous focus must not mutate history, and runtime failure must not strand the terminal in application mode.

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
- a modified key executes only when the configured keymap resolves it to deliberate emergency Quit.

No plain character is interpreted as a global or report command while editing. Outside edit mode, command routing remains unchanged.

## Edit persistence boundary

A draft changes canonical history only after explicit commit.

SQLite authority performs one fenced session-description update. Memory changes only after that transaction succeeds.

Legacy-file authority edits a cloned session collection, writes it through the existing persistence boundary, and replaces in-memory state only after the write succeeds.

A failed persistence attempt:

- does not alter canonical in-memory history;
- retains the complete draft and stable session ID;
- enters the existing visible persistence-recovery state;
- allows retry or authority reload under the established failure contract.

A successful commit closes edit mode. Cancel closes edit mode without a persistence write. Description edits do not invalidate daily sediment contributions because description text is not sediment-relevant chronology.

## Visible edit state

The report modal displays either:

- `VIEW · Enter edit · Esc back`; or
- `EDIT DESCRIPTION · Enter commit · Esc cancel`.

The selected row renders the live draft with an explicit cursor marker. Closing the report modal, changing UI mode, or resetting report state discards any uncommitted draft.

Deletion remains a separate configured command. Enter no longer exits the report-log view; it enters editing. Quit from report context returns an application exit decision instead of being silently ignored.

## Terminal lifecycle ownership

One `TerminalSession` RAII guard owns:

- raw-mode acquisition and release;
- alternate-screen entry and exit;
- cursor restoration;
- terminal-output flushing;
- the ratatui terminal instance;
- registration with the process-wide panic restoration hook.

Terminal restoration is idempotent. The guard marks cleanup complete before issuing restoration operations, so explicit restoration, `Drop`, and the panic hook cannot perform the lifecycle transition more than once.

Startup failures after partial acquisition use the same restoration boundary. Cleanup attempts every applicable step even if an earlier step fails and returns aggregated cleanup context.

## Panic custody

A process-wide panic hook is installed once. While a terminal session is active, the hook restores terminal state before delegating to the previously installed hook.

Panic restoration does not claim that application state or an emergency checkpoint was persisted. Panic output remains owned by the prior hook after the host terminal has been restored.

## Runtime I/O failure custody

Draw, event-poll, and event-read errors leave the inner application loop and enter one outer failure boundary.

Before returning the runtime error, Strata attempts one direct runtime checkpoint using the same validated checkpoint payload and SQLite/file authority paths as ordinary checkpointing. The emergency attempt remains fail-closed when checkpoint prerequisites are unavailable, including active recovery or queued mutations.

The returned `io::Error` preserves:

- the original error kind;
- the original runtime failure text;
- emergency checkpoint success or failure as appended context;
- terminal cleanup failure as appended context.

Checkpoint or cleanup failure cannot erase or replace the primary draw, poll, or read error.

Application finalization and terminal restoration are separate. Normal finalization errors remain primary while cleanup failure is attached as context.

## Exactly-once restoration certification

Linux pseudo-terminal process tests capture `stty -g` before and after each run and require equality. A debug-only restoration marker proves that restoration executes exactly once.

Certified paths include:

- normal quit;
- detach with checkpoint evidence;
- injected draw failure;
- injected poll failure;
- injected read failure;
- injected panic.

Runtime I/O failures must leave emergency checkpoint evidence when prerequisites are valid. Panic must restore the terminal without reporting an emergency checkpoint success claim.

Test-only fault and restoration-marker environment variables are active only in debug builds.

## Certification

INTERACTION-001A and 001B pass:

- formatting;
- strict Clippy with all targets/features and warnings denied;
- 170 unit tests;
- 9 CLI lifecycle tests;
- 6 configuration-authority tests;
- 1 report-help regression test;
- 12 SQLite/TUI process tests;
- 3 terminal-lifecycle PTY process tests covering six lifecycle paths;
- 2 temporal-authority tests;
- doc tests.

Focused lifecycle proofs cover idempotent restoration, preservation of primary error kind and text, application-error preservation when cleanup fails, termios restoration, emergency checkpoint publication, and panic cleanup without false persistence claims.

## Remaining interaction authority

INTERACTION-001C / issue #24 remains responsible for explicit Bound, Unbound, Disabled, contextual, and mandatory key semantics with command-atlas/runtime parity.
