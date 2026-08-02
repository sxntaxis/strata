---
id: RECONCILIATION-001B2B
kind: work
state: active
authority: working
created: 2026-08-02
updated: 2026-08-02
---

# RECONCILIATION-001B2B — legacy finish transition receipts

## Issue

Normal legacy-file exit currently mutates the active session in memory, then publishes session history, sediment state, daily contribution, and checkpoint deletion as separate operations.

Process death between those operations can leave a completed session row while the prior active checkpoint remains recoverable, duplicating time on restart. The legacy finish path also clears the active category description in memory but does not publish the category catalog before exit.

## Selected contract

### Prepared finish receipt

Before finalizing the active session in memory, capture the current active checkpoint and attach one deterministic finish receipt. The receipt binds:

- prior active category, description, and UTC start;
- canonical finish UTC;
- optional completed session row, present exactly when one or more whole seconds exist;
- no resulting active generation.

The prepared checkpoint is published before completed history or metadata effects. If it cannot be published, the active session remains unchanged.

### Publication and replay

Finish publication order is:

1. publish prior-generation checkpoint plus finish receipt;
2. finalize the active interval in memory;
3. publish session history;
4. publish cleared category-description state;
5. publish final sediment and daily-contribution authority;
6. remove the checkpoint receipt only after convergence.

Startup detects a finish receipt before ordinary active recovery. It validates the prior checkpoint generation, exact whole-second interval, session payload, and operation identity; replays missing session/catalog/sediment effects idempotently; then removes the receipt. It must not resume the finished active session.

### Scope

This unit covers normal legacy finish only. Clear-all/reset receipts, initial active-start/checkpoint coherence, and user-visible cutoff semantics remain later issue #10 work.

## Acceptance proofs

- prepared-receipt failure preserves the active session and old checkpoint;
- crash before session publication writes exactly one completed row on restart;
- crash after session publication exact-matches without duplication;
- crash after catalog/sediment publication converges and removes the receipt;
- failed later publication retains the receipt;
- zero-whole-second finish publishes no completed row;
- normal finish persists the cleared category description;
- restart never resumes a receipt-marked finished active generation;
- all prior switch receipt, recovery, sediment, CLI/TUI, and PTY suites remain green.
