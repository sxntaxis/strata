---
id: SEDIMENT-001C1
kind: work
state: active
authority: working
created: 2026-08-02
updated: 2026-08-02
---

# SEDIMENT-001C1 — compressed recovery mass

## Purpose

Establish the representation and arithmetic required for SEDIMENT-001C to recover arbitrarily long detached intervals without replaying or allocating one item per missed second.

## Problem

The current pending reservoir stores one category ID per blocked grain. Replacing physics replay with a bulk loop would still be linear in detached duration and could allocate millions of entries after a long gap. That is not bounded recovery.

## Selected contract

- Pending logical grains are stored as ordered category/count runs.
- Adjacent runs of the same category merge without losing FIFO category order.
- Bulk addition is constant in the number of added grains, apart from placement into currently free ingress columns.
- Flushing work is bounded by current ingress capacity, not pending mass.
- Snapshot size is proportional to physical grains plus category-run changes, not total blocked seconds.
- `SandState` schema version 2 serializes compressed runs.
- Version 1 states with `pending_grains` migrate deterministically into runs.
- Zero-count runs are ignored; unknown category IDs retain the established normalization to idle.
- Logical count overflow is rejected by the bulk API rather than wrapped or silently saturated.
- Periodic tick arithmetic calculates due event counts and remainders with integer nanosecond arithmetic, independent of elapsed duration.

## Acceptance proofs

- a billion pending grains require one compressed run;
- adjacent same-category additions merge, while category transitions preserve order;
- ingress flush performs at most one placement per free ingress column;
- version 1 pending vectors restore into equivalent version 2 runs;
- version 2 snapshot/restore preserves run order, counts, category identity, and total mass;
- category clearing/removal operates exactly across compressed runs;
- long-duration tick calculation returns exact due counts and accumulator remainder without iterative replay;
- all existing sediment, persistence, temporal, report, CLI, and TUI tests remain green.

## Boundary

This unit does not yet alter application startup, checkpoint claiming, recovery targets, or lifecycle behavior. SEDIMENT-001C2 will integrate these primitives into durable bounded checkpoint recovery and close issue #6.
