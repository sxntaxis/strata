---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-02
authority: working
summary: INTERACTION-001 is complete; editing, terminal lifecycle, key state, contextual routing, mandatory Quit, and atlas/runtime parity now have accepted authority.
next: Audit partially satisfied issues #5, #10, and #13 criterion by criterion against the merged SQLite implementation.
---

# NOW — Strata

## Current phase

The SQLite migration, startup authority, temporal, domain, reporting, sediment, and interaction programs are complete.

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
- primary-error preservation when checkpoint or cleanup also fails;
- explicit Bound, Unbound, and Disabled action state;
- one declared contextual key policy and resolver;
- mandatory recovery-safe Ctrl-C Quit separated from configurable keys;
- configurable F1 behavior without a physical-key bypass;
- command atlas and palette output derived from runtime reachability.

The project is entering **post-program issue reconciliation**.

## Verified technical baseline

- SQLite schema version 6 is authoritative after explicit activation.
- CLI and TUI share configuration, repository, temporal, session, recovery, snapshot, and interaction boundaries.
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
- Null physical-key entries unbind one key; `unbind_actions` explicitly disables actions.
- Contradictory Bound and Disabled configuration fails closed.
- Contextual aliases are named and conditionally resolved rather than embedded in handlers.
- Disabled actions are unreachable through direct keys, aliases, and the palette.
- Mandatory Ctrl-C uses persistence-recovery export custody before exit when recovery is active.
- Atlas rows and close/movement/jump hints use the same current bindings as runtime.

## Completed post-migration units

- **AUTHORITY-001** — issue #21.
- **TEMPORAL-001** — issue #25.
- **TEMPORAL-002** — issues #4, #23, #27.
- **DOMAIN-001** — issues #2, #12.
- **REPORT-001** — issues #1, #3, #14, #17, #28.
- **SEDIMENT-001** — issues #6, #7, #16, #18, #26.
- **INTERACTION-001A** — issue #19: explicit report-description edit mode.
- **INTERACTION-001B** — issue #20: terminal lifecycle guard and runtime failure custody.
- **INTERACTION-001C** — issue #24: explicit keymap state, contextual and mandatory policy, hidden-fallback removal, and atlas/palette/runtime parity.

## Active sequence

1. Reconcile partially satisfied issues #5, #10, and #13 against the merged SQLite authority and category archival implementation.
2. Later domain/UI distinction work under issue #22.
3. Later profile authority, including complete isolation and deliberate switching under issue #15.

## Current risks

- Issues #5, #10, and #13 may contain obsolete premises mixed with acceptance criteria already satisfied by SQLite; they require evidence-based reconciliation rather than assumption.
- Queued checkpoint mutations have no stable cross-authority receipt identity and fail closed.
- Complete profile switching/isolation remains open.
- Active draft versus category metadata remains an unresolved domain/UI distinction.

## Next

Audit **issues #5, #10, and #13** criterion by criterion. Map each criterion to current schema, repository, runtime, archival, recovery, and test evidence. Close only directly certified criteria; rewrite or retain any genuinely unresolved remainder.