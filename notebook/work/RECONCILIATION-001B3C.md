---
id: RECONCILIATION-001B3C
kind: work
state: accepted
authority: accepted
created: 2026-08-03
updated: 2026-08-03
---

# RECONCILIATION-001B3C — visible deterministic recovery cutoff

## Issue

Checkpoint recovery currently chooses and persists a fixed target before bounded reconstruction, but the user interface does not expose what time was durable, what interval was reconstructed, where reconstruction stopped, or which later interval remains provisional. A successful startup can therefore look exact even though part of the visible active interval was deterministically reconstructed from checkpoint evidence.

## Selected contract

Every successful checkpoint recovery produces one acknowledgment statement containing:

- checkpoint capture UTC;
- checkpoint simulation UTC, the last sediment instant represented directly by durable payload state;
- one recovery target UTC selected at claim time and persisted before reconstruction;
- reconstructed duration from simulation UTC through target UTC;
- active stable identity where SQLite owns one, category, description, and original active-session start UTC;
- recovered-interval classification: `exact` when no interval required reconstruction, otherwise `reconstructed`;
- post-target classification: `provisional live time`;
- deterministic cutoff policy: no time after the persisted target is counted as recovered, and retry reuses the persisted target rather than extending it.

The statement blocks ordinary controls until acknowledged with Enter or Esc. Mandatory emergency quit remains available. A persistence-failure overlay has higher priority and retains the same statement for later acknowledgment.

The emergency recovery export includes the same structured statement when present.

## Acceptance proofs

- exact recovery shows zero reconstructed duration and `exact` classification;
- nonzero bounded recovery shows `reconstructed` and the exact duration;
- capture, simulation, target, and active start must be monotonic and fail closed otherwise;
- retry of an already claimed checkpoint reuses the persisted target;
- the visible statement and emergency export carry identical values;
- normal controls remain blocked until acknowledgment;
- post-target time is explicitly labeled provisional and is never folded into the recovered duration;
- repeated startup after committed recovery cannot silently extend the original cutoff;
- all recovery, receipt, sediment, SQLite/TUI, CLI, and PTY suites remain green.

## Closure condition

This is the final bounded unit for issue #10. The issue closes only after the implementation, process-level restart proof, durable authority promotion, and exact-head CI all pass.


## Implemented result

- successful checkpoint recovery builds one structured evidence statement from the claimed checkpoint and persisted target;
- chronology fails closed unless active start, durable simulation, capture, and target are monotonic;
- the statement exposes active identity, category, description, start, capture, durable simulation, target, reconstructed duration, recovered classification, post-target classification, and cutoff policy;
- exact zero-duration recovery is distinguished from reconstructed recovery;
- post-target time is always labeled `provisional live time` and is not folded into recovered history;
- ordinary controls remain blocked until Enter or Esc acknowledgment, while mandatory emergency quit and higher-priority persistence recovery remain available;
- a failed recovery commit retains the target in recovering evidence; delayed retry reuses and displays that original cutoff;
- emergency recovery export schema 3 carries the same structured statement and classifications.

## Certification

- formatting: pass;
- strict Clippy, all targets/features, warnings denied: pass;
- 228 library tests: pass;
- 9 CLI lifecycle process tests: pass;
- 6 configuration-authority tests: pass;
- 1 report-help regression test: pass;
- 14 SQLite/TUI process tests: pass;
- 2 temporal-authority tests: pass;
- 3 terminal-lifecycle PTY process tests: pass;
- repeated failed-commit/delayed-retry process proof: pass;
- emergency export schema/value parity proof: pass;
- temporary transformation, audit, and workflow machinery: absent from the permanent tree.

RECONCILIATION-001B3C completes issue #10. Crash recovery now has evidence-backed identity, atomicity, replay, bounded reconstruction, exact transition edges, deterministic cutoff reuse, visible uncertainty, acknowledgment custody, and export parity.
