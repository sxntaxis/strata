---
id: RECONCILIATION-001B3A
kind: work
state: active
authority: working
created: 2026-08-03
updated: 2026-08-03
---

# RECONCILIATION-001B3A — atomic initial active generation

## Issue

Under SQLite authority, first TUI startup with no existing active generation currently commits the `active_session` row before restoring sediment and publishing the first runtime checkpoint. Process death or a later startup failure can therefore expose an active generation with no matching recoverable checkpoint evidence.

## Selected contract

- A new SQLite TUI active generation and its first pending runtime checkpoint form one immediate transaction.
- The checkpoint binds the same stable active identity, UTC start, category, description, simulation timestamp, accumulators, and canonical sediment state staged in memory.
- Existing active state or any pre-existing checkpoint blocks initial bootstrap; startup never overwrites unresolved evidence.
- Sediment restoration succeeds before the transaction is attempted.
- Failure before write, after active insertion, after checkpoint insertion, or immediately before commit leaves neither row durable.
- Once committed, restart always observes both authorities under the same stable identity.
- Legacy-file startup remains a single atomic checkpoint-file publication and does not gain a second competing active authority.

## Acceptance proofs

- successful initial bootstrap creates exactly one active row and one pending checkpoint with the same stable ID;
- every injected SQLite bootstrap failure rolls both rows back;
- the real TUI startup path fails visibly without leaving an orphan generation and can be retried cleanly;
- existing active/checkpoint recovery paths remain unchanged;
- formatting, strict Clippy, all tests, and process suites remain green.

## Boundary

This unit closes only the initial active-start/checkpoint window. Exact remaining transition-edge sediment attribution and visible recovery cutoff/reconstruction semantics remain later issue #10 units.
