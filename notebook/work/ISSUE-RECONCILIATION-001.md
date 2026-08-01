---
id: ISSUE-RECONCILIATION-001
kind: work
state: active
authority: working
created: 2026-08-01
updated: 2026-08-01
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
| #4, #23, #27 | Interval-boundary allocation, misleading sunrise semantics, and zero-duration policy are the next coupled temporal risks. | TEMPORAL-002 |
| #2, #12 | SQLite preserves project strings, but the complete project/classification product contract must be reconciled before closure. | DOMAIN-001 |
| #1, #14, #17, #28, #3 | Reporting and export semantics/documentation. | REPORT-001 |
| #5, #10, #13 | SQLite integrity, active authority, and category archival likely satisfy substantial portions; verify every criterion before closing or rewriting. | reconciliation audit |
| #6, #7, #16, #18, #26 | Sediment conservation/topology/rendering remain conceptually coupled. | SEDIMENT-001 |
| #19, #20, #24 | Interaction modes, terminal cleanup, and keymap truth remain independent of SQLite. | INTERACTION-001 |
| #22 | Active draft versus category metadata remains a domain/UI distinction. | DOMAIN-002 |

## Immediate action

Implement TEMPORAL-002 for issues #4, #23, and #27. Clock authority is now explicit; the remaining question is how truthful intervals are divided at boundaries, named, and represented when their duration is zero.
