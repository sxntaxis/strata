---
id: RELIABILITY-001
kind: work
state: active
created: 2026-08-01
updated: 2026-08-01
authority: working
summary: Bounded campaign sequencing SQLite migration and confirmed audit defects without allowing the storage rewrite to freeze unresolved product semantics.
---

# RELIABILITY-001 — Persistence and audit remediation

## Governing outcome

Make Strata dependable enough that exact chronological history and accountable sedimentary history survive normal use, concurrent interfaces, failures, restarts, migration, and recovery without silent loss, duplication, remapping, or false success.

## Present authority

- Accepted persistence direction: `docs/ARCHITECTURE.md` and GitHub issue #8.
- Accepted product identity: `docs/PROJECT.md`.
- Confirmed defects and proposed solutions: GitHub issues #2–#28.
- Unresolved product semantics: `CONCEPT-001` and the Notebook decision register.
- Current implementation reality: Rust source, tests, CI, and observed runtime.

## Constraints

- Do not translate the current CSV schema directly into SQLite without a domain review.
- Do not let SQLite migration hide product-semantic defects.
- Do not combine all remediation into one unreviewable pull request.
- Preserve legacy data and provide a verified rollback/recovery path.
- TUI and CLI must share one authority rather than maintain parallel lifecycle semantics.
- Sediment conservation and recovery are product requirements, not optional visualization polish.
- No issue is closed by documentation or schema presence alone; runtime proof is required.

## Non-goals

- A hosted service or mandatory daemon.
- Cloud synchronization.
- A universal event-sourcing architecture.
- Replacing the TUI artistic language.
- Resolving every conceptual question inside the database migration.
- Maintaining CSV and SQLite as indefinite competing live authorities.

## Sequence

### R0 — Concept gates

Resolve before final schema design:

- deliberate detach and unexpected-termination semantics;
- active, idle, and inferred interval states;
- flat layer versus primary layer plus context;
- minimum formation identity and temporal-quantum fields;
- reconstruction provenance required by sediment recovery.

Receiving work: `CONCEPT-001`.

### R1 — Persistence contract and failure behavior

Issues:

- #9 ignored persistence failures;
- #10 crash-lost active TUI interval;
- #11 non-idempotent CLI start/stop;
- #15 profile/data-path mixing;
- #21 invalid-config fallback;
- relevant additions to #6.

Outputs:

- repository API shared by CLI and TUI;
- typed transaction outcomes;
- one-active-interval invariant;
- profile/database identity;
- visible failure and emergency-recovery behavior;
- process-level fault-injection plan.

### R2 — SQLite schema and verified legacy migration

Primary issue: #8.

Related domain issues:

- #2 project identity;
- #4 operational-day overlap;
- #5 orphan category references;
- #12 default idle classification;
- #13 layer retirement;
- #22 session-description lifecycle;
- #25 time and timezone authority;
- #27 zero-duration rows.

Outputs:

- schema and migration ledger;
- absolute timestamp model;
- stable layer and interval identities;
- active/recovery state model;
- category archival and reference constraints;
- deterministic validated import;
- legacy backup and reconciliation report;
- database integrity and backup operations.

### R3 — Reporting, export, and interaction consistency

Issues:

- #1 custom inclusive ranges;
- #3 report wording;
- #14 active-session reporting;
- #17 ICS correctness;
- #19 report-detail editing mode;
- #23 sunrise claim;
- #24 unbinding semantics;
- #28 deterministic ordering.

Outputs:

- one report semantics model shared across CLI and TUI;
- explicit provisional active intervals;
- parser-validated exports;
- visible edit/commit/cancel boundaries;
- truthful configuration and help contracts.

### R4 — Sediment conservation and formation recovery

Issues:

- #6 detach/catch-up;
- #7 resize topology loss;
- #16 lost spawn ticks;
- #18 conflicting snapshot meanings;
- #26 renderer width misuse.

Outputs:

- logical formation model independent of viewport capacity;
- exact total and per-layer mass conservation;
- explicit topology preservation tolerance;
- deterministic bounded deposition for inferred time;
- immutable historical viewing;
- separate cumulative, daily, and derived snapshot kinds;
- exact dimension vocabulary and rendering bounds.

### R5 — Runtime lifecycle hardening

Issues:

- #20 terminal restoration and final handling;
- remaining failure-injection, concurrency, and recovery gaps from #9–#11 and #25.

Outputs:

- RAII terminal restoration;
- process interruption tests;
- concurrent CLI/TUI tests;
- clock-jump and suspend tests;
- verified recovery after failed commit and interrupted migration.

## Pull-request discipline

Each implementation PR must declare:

- exact issue set;
- product and Notebook authority read;
- schema/runtime boundary;
- data migration effect;
- failure and rollback behavior;
- tests added before closure;
- observed limitations;
- next bounded unit.

## Completion criteria

RELIABILITY-001 completes only when:

- SQLite is the sole live authority;
- legacy import is verified and recoverable;
- active and inferred intervals are crash-safe and idempotent;
- CLI and TUI converge on one lifecycle and report model;
- exact chronological history survives faults and concurrency;
- sediment mass and accepted structural properties survive resize, detach, recovery, and migration;
- exports are parser-validated;
- no critical write or configuration failure is silent;
- process-level and fault-injection tests cover the critical paths;
- unresolved conceptual questions are either accepted, explicitly deferred without schema lock-in, or refused.

## Current next edge

Complete R0 decision gates for detach semantics and the layer/context model. Then derive the smallest SQLite schema proposal that can represent accepted meaning without implementing the migration yet.
