---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-02
authority: working
summary: INTERACTION-001A is complete; report-log viewing and historical-description editing are explicit separate modes.
next: Implement INTERACTION-001B for issue #20: process-wide terminal restoration and runtime failure custody.
---

# NOW — Strata

## Current phase

The SQLite migration, authority, temporal, domain, reporting, and sediment programs are complete. INTERACTION-001A has established explicit report-log edit ownership and atomic draft persistence.

Strata now has:

- durable fail-closed persistence;
- explicit monotonic/UTC/fixed-offset time authority;
- canonical project/category/session identity;
- truthful deterministic reports and exports;
- conserved sediment mass, topology, recovery, snapshots, and daily contributions;
- explicit separation between report-log viewing and description editing;
- stable-ID drafts that become canonical only after one successful commit;
- visible edit/cancel/commit state;
- command-letter and Unicode text support inside edit mode without stealing commands outside it.

The project is implementing **interaction authority**.

## Verified technical baseline

- SQLite schema version 6 is authoritative after explicit activation.
- CLI and TUI share configuration, repository, temporal, session, recovery, and snapshot boundaries.
- Historical sediment viewing is immutable and typed daily contributions are revision-matched to ledger truth.
- Report-log view is read-only.
- Confirm creates a draft owned by a stable persisted session ID.
- Plain command letters, spaces, and Unicode are draft text only while editing.
- Enter requests one commit; Esc cancels the whole draft.
- SQLite or legacy-file persistence succeeds before in-memory history changes.
- Failed commit retains the complete draft and enters visible persistence recovery.
- Description-only edits do not invalidate sediment contributions.
- Quit from report context is no longer silently ignored.

## Completed post-migration units

- **AUTHORITY-001** — issue #21.
- **TEMPORAL-001** — issue #25.
- **TEMPORAL-002** — issues #4, #23, #27.
- **DOMAIN-001** — issues #2, #12.
- **REPORT-001** — issues #1, #3, #14, #17, #28.
- **SEDIMENT-001** — issues #6, #7, #16, #18, #26.
- **INTERACTION-001A** — issue #19: explicit report-description edit mode and atomic draft commit.

## Active sequence

1. **INTERACTION-001B** — issue #20: process-wide terminal lifecycle restoration and runtime-error recovery custody.
2. **INTERACTION-001C** — issue #24: explicit keymap state and command-atlas/runtime parity.
3. Reconcile partially satisfied issues #5, #10, and #13.
4. Later domain/profile work, including issue #15 and issue #22.

## Current risks

- Raw mode, alternate screen, mouse capture, and cursor restoration lack one process-wide RAII owner.
- Draw, poll, read, and panic paths can bypass terminal cleanup or emergency checkpoint publication.
- Keybinding configuration and runtime behavior still contain hidden fallbacks and direct F1 handling.
- Queued checkpoint mutations have no stable cross-authority receipt identity and fail closed.
- Complete profile switching/isolation remains open.

## Next

Implement **INTERACTION-001B** for issue #20. Introduce one terminal lifecycle guard, separate host-terminal restoration from application finalization, preserve original runtime errors, attempt emergency checkpoint custody, and certify normal quit, detach, runtime failure, and panic behavior under PTY tests.
