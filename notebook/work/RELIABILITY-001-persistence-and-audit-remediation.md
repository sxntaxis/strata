---
id: RELIABILITY-001
kind: work
state: active
authority: working
created: 2026-08-01
updated: 2026-08-01
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

## Remaining order

1. **TEMPORAL-002** — issues #4, #23, #27.
2. **DOMAIN-001** — issues #2 and #12 residuals.
3. **REPORT-001** — issues #1, #14, #17, #28.
4. **SEDIMENT-001** — issues #6, #7, #16, #18, #26.
5. **INTERACTION-001** — issues #19, #20, #24.

## Closure discipline

Each issue must be reconciled against current main. Close only when every acceptance criterion is supported by merged behavior or when the issue is explicitly rewritten to isolate the residual defect.
