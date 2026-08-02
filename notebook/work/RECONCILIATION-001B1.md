---
id: RECONCILIATION-001B1
kind: work
state: active
authority: working
created: 2026-08-02
updated: 2026-08-02
---

# RECONCILIATION-001B1 — active identity and checkpoint coherence

## Issue

Issue #10 is substantially improved by the SQLite active-session and bounded checkpoint programs, but crash-window reconciliation found remaining contradictions:

- a SQLite switch/reset can commit a new active stable ID while leaving a pending checkpoint bound to the previous ID;
- the next checkpoint save then conflicts, and restart may claim stale evidence against the replacement active row;
- legacy switches/resets do not refresh the runtime checkpoint immediately, so a crash can replay the pre-transition active session;
- active description changes can remain newer than the checkpoint used on restart;
- normal finish can leave stale checkpoint evidence if the process dies after the session transaction but before ordinary cleanup.

## Selected contract

### Transition boundary

Every successful active start, switch, reset, description update, and finish must leave active-session authority and checkpoint evidence in one coherent generation.

- SQLite switch/reset retires checkpoint evidence for the expected prior stable ID inside the same transaction as the active-row transition.
- SQLite finish retires checkpoint evidence for the completed stable ID inside the same transaction as the completed-session write and active-row removal.
- A recovering or quarantined checkpoint cannot be silently retired by an ordinary transition.
- Checkpoint identity mismatches fail closed before active history changes.
- After a successful switch/reset/description update, the application immediately publishes a fresh checkpoint for the current active identity before accepting further input.

### Legacy boundary

Legacy-file authority immediately replaces the runtime checkpoint after every successful active switch, reset, or active-description persistence. It must never leave a known stale pre-transition payload as the latest recoverable evidence.

This unit eliminates stale replay and checkpoint conflicts. A later B2 unit will certify the remaining unavoidable multi-file crash window with a stable legacy transition receipt and make the recovery cutoff policy explicitly visible.

### Startup validation

SQLite startup must reject or quarantine checkpoint evidence whose active stable ID does not match the authoritative active row. It may not install the checkpoint identity into memory and discover the conflict only during recovery commit.

## Acceptance proofs

- switch with a pending checkpoint cannot leave the old stable ID recoverable;
- reset with a pending checkpoint cannot leave the old stable ID recoverable;
- finish with a pending checkpoint cannot resurrect a completed active session;
- recovering/quarantined evidence blocks ordinary transition retirement;
- checkpoint replacement failure after a successful transition enters visible persistence recovery;
- legacy switch/reset/description paths refresh evidence immediately;
- startup rejects mismatched active/checkpoint identity before applying sediment recovery;
- all prior runtime coordination, checkpoint, sediment, interaction, category, CLI, TUI, and PTY gates remain green.

## Boundary

This unit does not yet claim full issue #10 closure. RECONCILIATION-001B2 must add stable legacy transition receipts, certify kill-between-file-publications, and expose the chosen recovery cutoff policy to the user.