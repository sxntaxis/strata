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
| #21 | Completed by AUTHORITY-001: shared fail-closed startup configuration. | none |
| #25 | Completed by TEMPORAL-001: explicit clock and civil-time authority. | none |
| #15 | Complete profile identity, isolation, and deliberate runtime switching remain open. | AUTHORITY-002 or later profile unit |
| #4, #23, #27 | Completed by TEMPORAL-002: overlap allocation, fixed-clock truth, and zero-transition policy. | none |
| #2, #12 | Completed by DOMAIN-001: project/category identity, explicit classification, and idle vocabulary. | none |
| #1, #3, #14, #17, #28 | Completed by REPORT-001: truthful ranges/help, provisional active projection, valid ICS, and deterministic ordering. | none |
| #5, #10, #13 | SQLite integrity, active authority, and category archival likely satisfy substantial portions; verify every criterion before closing or rewriting. | reconciliation audit |
| #6 | Completed by SEDIMENT-001C1/C2: compressed mass and bounded topology-preserving recovery. | none |
| #7 | Completed by SEDIMENT-001B: canonical logical canvas and projection-only resize. | none |
| #16, #26 | Completed by SEDIMENT-001A: lossless ingress, explicit geometry units, and exact Braille viewport dimensions. | none |
| #18 | Completed by SEDIMENT-001D1/D2: typed snapshot identity, immutable viewing, revision-matched daily contributions, mutation/recovery reconciliation, and legacy evidence custody. | none |
| #19, #20, #24 | Interaction modes, terminal cleanup, and keymap truth remain independent of SQLite. | INTERACTION-001 |
| #22 | Active draft versus category metadata remains a domain/UI distinction. | DOMAIN-002 |

## Immediate action

Implement INTERACTION-001 for issues #19, #20, and #24. Establish explicit edit-mode ownership, process-wide terminal restoration, and certified parity between configured bindings and runtime behavior.
