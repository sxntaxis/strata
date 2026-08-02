---
id: ISSUE-RECONCILIATION-001
kind: work
state: active
authority: working
created: 2026-08-01
updated: 2026-08-02
---

# ISSUE-RECONCILIATION-001 — post-SQLite queue

The original issue descriptions predate the completed SQLite migration. Their acceptance criteria remain useful, but code-path premises must be reverified against every still-supported authority.

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
| #5 | Completed by RECONCILIATION-001A: malformed or unknown legacy session category IDs fail closed, retain the original value for repair, and are never reinterpreted as idle. | none |
| #10 | Checkpoint, active-session, finish-receipt, and runtime-recovery work likely satisfy most criteria; verify each crash window and user-visible recovery guarantee. | RECONCILIATION-001B |
| #13 | Historical data-loss defect completed by RECONCILIATION-001A: active/archived metadata, reports, sand, tags, restore, and migration retain stable meaning under SQLite and legacy authority. Remaining scope is explicit merge/reassignment plus permanent-deletion policy and tests. | DOMAIN-002 or dedicated category-merge unit |
| #6, #7, #16, #18, #26 | Completed by SEDIMENT-001: conserved mass/topology/recovery and truthful immutable historical artifacts. | none |
| #19 | Completed by INTERACTION-001A: explicit report-log edit mode, stable-ID draft, atomic commit, full cancel, and command/text separation. | none |
| #20 | Completed by INTERACTION-001B: exactly-once terminal restoration, runtime emergency checkpoint custody, primary-error preservation, and PTY certification. | none |
| #24 | Completed by INTERACTION-001C: explicit Bound/Unbound/Disabled state, declared contextual aliases, mandatory recovery-safe Ctrl-C, configurable F1, hidden-fallback removal, and atlas/palette/runtime parity. | none |
| #22 | Active draft versus category metadata remains a domain/UI distinction. | DOMAIN-002 |

## Immediate action

Audit issue #10 criterion by criterion against active-session persistence, detached checkpoints, finish receipts, recovery claims, emergency checkpointing, and user-visible recovery behavior. After that, define the merge/reassignment transaction required to complete issue #13 without reopening its resolved historical-integrity defect.