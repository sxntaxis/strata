---
id: ISSUE-RECONCILIATION-001
kind: work
state: active
authority: working
created: 2026-08-01
updated: 2026-08-02
---

# ISSUE-RECONCILIATION-001 — post-SQLite queue

The original issue descriptions predate the completed SQLite migration. Their acceptance criteria remain useful, but code-path premises must be reverified.

## Current disposition

| Issues | Disposition | Next owner |
|---|---|---|
| #8, #9, #11 | Completed and closed by the SQLite program. | none |
| #21 | Completed by AUTHORITY-001: CLI/TUI share one fail-closed startup configuration gate with explicit `--ignore-config`. | none |
| #25 | Completed by TEMPORAL-001: monotonic live duration, checked UTC recovery, fixed-offset civil authority, clock-jump refusal, and persisted historical day grouping. | none |
| #15 | Complete profile identity, isolation, and deliberate runtime switching remain open. | AUTHORITY-002 or a later profile unit |
| #4, #23, #27 | Completed by TEMPORAL-002: canonical-session overlap allocation, visible removal/migration of false sunrise semantics, and receipt-only zero transitions. | none |
| #2, #12 | Completed by DOMAIN-001: project survives canonical history and exports; CLI classification is explicit; idle is deliberate and user-facing. | none |
| #1, #14, #17, #28, #3 | Reporting and export semantics/documentation. | REPORT-001 |
| #5, #10, #13 | SQLite integrity, active authority, and category archival likely satisfy substantial portions; verify every criterion before closing or rewriting. | reconciliation audit |
| #6, #7, #16, #18, #26 | Sediment conservation/topology/rendering remain conceptually coupled. | SEDIMENT-001 |
| #19, #20, #24 | Interaction modes, terminal cleanup, and keymap truth remain independent of SQLite. | INTERACTION-001 |
| #22 | Active draft versus category metadata remains a domain/UI distinction. | DOMAIN-002 |

## Immediate action

Implement REPORT-001 for issues #1, #14, #17, and #28. Canonical ledger, interval, project, and category truth are now stable; reporting and export projections must become complete without rewriting them.
