---
id: RELIABILITY-001
kind: work
state: active
authority: working
created: 2026-08-01
updated: 2026-08-01
---

# RELIABILITY-001 — persistence and audit remediation

## Completed program: SQLITE-001 through SQLITE-012

The persistence migration is complete:

- schema, strict legacy import, migration publication, repository parity;
- deterministic interchange, doctor, backup, and restore;
- explicit activation and CLI/TUI authority cutover;
- runtime fencing, receipts, checkpoint recovery;
- visible persistence recovery and exhaustive fault certification;
- validation-only import and legacy-evidence custody;
- final 9/9 closure audit and issue #8 closure.

SQLite work must not continue merely because the prefix existed. New units should be named for the domain risk they resolve.

## Post-migration order

1. **AUTHORITY-001** — issue #21 and profile-isolation foundations from #15.
2. **TEMPORAL-001** — issue #25.
3. **TEMPORAL-002** — issues #4, #23, #27.
4. **DOMAIN-001** — issues #2 and #12 residuals.
5. **REPORT-001** — issues #1, #14, #17, #28.
6. **SEDIMENT-001** — issues #6, #7, #16, #18, #26.
7. **INTERACTION-001** — issues #19, #20, #24.

## Closure discipline

Each issue must be reconciled against current main. Close only when every acceptance criterion is supported by merged behavior or when the issue is explicitly rewritten to isolate the residual defect.
