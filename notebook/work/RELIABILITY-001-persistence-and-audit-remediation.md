---
id: RELIABILITY-001
kind: work
state: active
authority: working
created: 2026-08-01
updated: 2026-08-02
---

# RELIABILITY-001 — persistence and audit remediation

## Completed persistence program

SQLITE-001 through SQLITE-012 completed schema, strict legacy import, explicit activation, shared CLI/TUI authority, transactional runtime coordination, deterministic interchange, maintenance, visible recovery, exhaustive fault certification, and legacy-evidence custody. Issue #8 closed at 9/9 acceptance criteria.

## Completed authority units

### AUTHORITY-001 — issue #21

- one validated startup configuration is shared by CLI and TUI;
- invalid authority/time settings fail before writable authority opens;
- `--ignore-config` is the explicit deliberate-default bypass;
- TUI reload retains the last valid settings on failure.

### TEMPORAL-001 — issue #25

- live duration is monotonic and committed with the same elapsed value;
- persisted absolute timestamps are UTC;
- live wall/monotonic skew above five seconds blocks mutation and preserves active state;
- future timestamps and unrepresentable monotonic reconstruction fail visibly;
- cross-process intervals above seven days require explicit CLI confirmation;
- configured fixed offset owns new civil projection and operational-day allocation;
- persisted operational-day keys own historical report grouping after later setting changes;
- the policy is explicitly fixed-offset, not IANA/DST.

Profile switching remains separate under issue #15.


### TEMPORAL-002 — issues #4, #23, #27

- canonical sessions remain single ledger identities;
- reports allocate exact overlap slices using fixed-offset boundary provenance captured per session;
- cross-boundary seconds are conserved and exact-boundary endpoints create no empty fragments;
- the false sunrise mode is removed and existing configuration migrates visibly to fixed-clock policy;
- zero-whole-second finishes and switches create transactional receipts without completed work rows;
- SQLite schema version 5, bundle schema version 2, and backward-compatible legacy CSV preserve the policy.

### DOMAIN-001 — issues #2 and #12

- project identity is independent from category and persists through legacy and SQLite lifecycle paths;
- shared domain sessions, TUI synchronization, custody export, JSON, and ICS retain project;
- CLI starts require explicit category classification before mutation;
- idle is the user-facing category-0 name and remains excluded from ordinary active totals;
- old `none`/`drift` names remain compatibility aliases rather than product vocabulary;
- legacy 8-, 12-, and 13-column CSV generations are handled explicitly.

## Remaining order

1. **REPORT-001** — issues #1, #14, #17, #28.
2. **SEDIMENT-001** — issues #6, #7, #16, #18, #26.
3. **INTERACTION-001** — issues #19, #20, #24.

## Closure discipline

Each issue must be reconciled against current main. Close only when every acceptance criterion is supported by merged behavior or when the issue is explicitly rewritten to isolate the residual defect.
