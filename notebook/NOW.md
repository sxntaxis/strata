---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-02
authority: working
summary: INTERACTION-001A and 001B are complete; editing and host-terminal lifecycle now have explicit authority.
next: Implement INTERACTION-001C for issue #24: keymap state, contextual policy, and atlas/runtime parity.
---

# NOW — Strata

## Current phase

The SQLite migration, authority, temporal, domain, reporting, sediment, explicit editing, and terminal-lifecycle units are complete.

Strata now has:

- durable fail-closed persistence;
- explicit monotonic/UTC/fixed-offset time authority;
- canonical project/category/session identity;
- truthful deterministic reports and exports;
- conserved sediment mass, topology, recovery, snapshots, and daily contributions;
- explicit report-log view/edit ownership and atomic draft persistence;
- one process-wide RAII terminal owner;
- exactly-once raw-mode, alternate-screen, cursor, and output restoration;
- emergency checkpoint custody for draw, poll, and read failures;
- panic restoration without false persistence claims;
- primary-error preservation when checkpoint or cleanup also fails.

The project is completing **interaction authority**.

## Verified technical baseline

- SQLite schema version 6 is authoritative after explicit activation.
- CLI and TUI share configuration, repository, temporal, session, recovery, and snapshot boundaries.
- Historical sediment viewing is immutable and daily contributions are revision-matched to ledger truth.
- Report-log view is read-only; Confirm creates a stable-ID edit draft.
- Plain command letters, spaces, and Unicode are text only inside edit mode.
- Enter commits once; Esc cancels the complete draft.
- Persistence succeeds before in-memory history changes.
- Failed edit commit retains the draft and visible recovery state.
- `TerminalSession` owns raw mode, alternate screen, cursor restoration, output flushing, and ratatui terminal state.
- Startup failure, normal close, Drop, and panic converge on idempotent restoration.
- Draw, poll, and read errors attempt direct emergency checkpoint publication.
- Runtime error kind and text remain primary; checkpoint and cleanup outcomes are appended context.
- Linux PTY tests certify unchanged termios state and exactly one restoration on quit, detach, draw/poll/read failure, and panic.

## Completed post-migration units

- **AUTHORITY-001** — issue #21.
- **TEMPORAL-001** — issue #25.
- **TEMPORAL-002** — issues #4, #23, #27.
- **DOMAIN-001** — issues #2, #12.
- **REPORT-001** — issues #1, #3, #14, #17, #28.
- **SEDIMENT-001** — issues #6, #7, #16, #18, #26.
- **INTERACTION-001A** — issue #19: explicit report-description edit mode.
- **INTERACTION-001B** — issue #20: terminal lifecycle guard and runtime failure custody.

## Active sequence

1. **INTERACTION-001C** — issue #24: explicit keymap state, contextual policy, mandatory controls, and command-atlas/runtime parity.
2. Reconcile partially satisfied issues #5, #10, and #13.
3. Later domain/profile work, including issue #15 and issue #22.

## Current risks

- Keybinding configuration and runtime behavior still contain hidden Confirm, Cancel, and ReportToday fallbacks.
- Physical F1 bypasses configured action state.
- Contextual aliases are executed by scattered modal logic rather than one declared policy.
- The command atlas can display keys that are not reachable under the current configuration.
- Queued checkpoint mutations have no stable cross-authority receipt identity and fail closed.
- Complete profile switching/isolation remains open.

## Next

Implement **INTERACTION-001C** for issue #24. Preserve explicit Bound, Unbound, and Disabled action state, declare mandatory emergency controls separately, route contextual behavior through one policy, remove hidden fallbacks, and certify that the command atlas matches actual runtime reachability.
