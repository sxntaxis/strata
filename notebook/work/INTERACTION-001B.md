---
id: INTERACTION-001B
kind: work
state: completed
authority: accepted
created: 2026-08-02
updated: 2026-08-02
---

# INTERACTION-001B — terminal lifecycle guard

## Issue

Issue #20: draw, event-poll, event-read, and panic failures could leave raw mode or alternate-screen state active and bypass emergency recovery custody.

## Accepted contract

- One RAII `TerminalSession` owns raw mode, alternate-screen state, cursor restoration, output flushing, and the ratatui terminal.
- Restoration is idempotent and executes exactly once through explicit close, `Drop`, partial-startup failure, or the process-wide panic hook.
- Cleanup attempts every applicable restoration step even if an earlier step fails.
- The panic hook restores terminal state before delegating to the prior hook and does not claim application persistence success.
- Application finalization remains separate from host-terminal restoration.
- Draw, poll, and read errors leave the application loop through one outer lifecycle boundary.
- Runtime I/O failure attempts one direct emergency runtime checkpoint.
- Returned errors preserve the original `io::ErrorKind` and primary text; checkpoint and cleanup outcomes are appended as context.
- Debug-only fault injection and restoration markers certify real process behavior without becoming release features.

## Certified proofs

- restore-once state is idempotent under explicit close plus `Drop`;
- primary runtime error kind and text survive checkpoint and cleanup failures;
- application finalization errors remain primary when cleanup also fails;
- normal quit restores PTY termios state exactly once;
- detach restores PTY termios state exactly once and leaves checkpoint evidence;
- injected draw, poll, and read faults restore PTY state and leave emergency checkpoint evidence;
- panic restores PTY state exactly once without an emergency-checkpoint success claim;
- all prior persistence, sediment, report, edit-mode, CLI, and TUI tests remain green;
- formatting and strict Clippy pass with all targets and features.

## Durable authority

- `docs/INTERACTION_AUTHORITY.md` records terminal ownership and runtime failure custody;
- `docs/ARCHITECTURE.md` assigns host-terminal lifecycle to `src/app/terminal_lifecycle.rs`;
- STRATA-D036 and STRATA-D037 constrain restoration and error composition;
- `notebook/work/INTERACTION-001.md` advances to keymap truth.

## Boundary

This unit does not redefine configured, unbound, disabled, contextual, or mandatory key semantics. Those remain INTERACTION-001C / issue #24.
