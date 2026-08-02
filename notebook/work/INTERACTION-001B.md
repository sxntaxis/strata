---
id: INTERACTION-001B
kind: work
state: active
authority: working
created: 2026-08-02
updated: 2026-08-02
---

# INTERACTION-001B — terminal lifecycle guard

## Issue

Issue #20: draw, event-poll, event-read, and panic failures can return or unwind after raw mode and alternate-screen entry but before terminal restoration or an emergency recovery transition.

## Selected contract

- One RAII `TerminalSession` owns raw mode, alternate-screen entry, cursor restoration, and the ratatui terminal.
- Restoration is idempotent and runs exactly once through explicit close, `Drop`, or the process-wide panic hook.
- The panic hook restores terminal state before delegating to the prior hook; it does not claim application persistence success.
- Application finalization remains separate from host-terminal restoration.
- A draw, poll, or read error returns to the outer lifecycle boundary.
- Before returning a runtime I/O error, Strata attempts one direct emergency runtime checkpoint.
- The returned error preserves the original failure and appends checkpoint and cleanup outcomes as context.
- Cleanup attempts every restoration step even if an earlier step fails.
- Startup failures after partial terminal acquisition also run the same restoration boundary.
- Test-only fault injection is available only in debug builds.

## Acceptance proofs

- restore-once state is idempotent under explicit close plus `Drop`;
- primary runtime error text and kind survive checkpoint/cleanup failures;
- draw, poll, and read faults restore PTY termios state;
- runtime faults leave an emergency checkpoint when checkpoint prerequisites are valid;
- normal quit and detach restore PTY termios state;
- panic restores PTY termios state without claiming emergency checkpoint success;
- all prior persistence, sediment, report, edit-mode, CLI, and TUI tests remain green.

## Boundary

This unit does not redefine configured, unbound, disabled, contextual, or mandatory key semantics. Those remain INTERACTION-001C / issue #24.
