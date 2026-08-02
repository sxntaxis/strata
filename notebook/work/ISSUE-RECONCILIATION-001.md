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
| #10 | Partially completed by RECONCILIATION-001B1, B2A, and B2B: SQLite active/checkpoint generations are transactional; legacy switch and normal finish publish prepared receipts and replay every certified multi-file kill point idempotently; finish converges session, catalog, sediment, and every affected daily contribution; recovery reload/flush/export preserve archived meaning. Remaining scope is clear-all/reset receipt custody, initial active-start/checkpoint coherence, exact transition-edge sediment reconciliation, and explicit recovery-cutoff/uncertainty presentation. | RECONCILIATION-001B2C |
| #13 | Historical data-loss defect completed by RECONCILIATION-001A: active/archived metadata, reports, sand, tags, restore, and migration retain stable meaning under SQLite and legacy authority. Remaining scope is explicit merge/reassignment plus permanent-deletion policy and tests. | DOMAIN-002 or dedicated category-merge unit |
| #6, #7, #16, #18, #26 | Completed by SEDIMENT-001: conserved mass/topology/recovery and truthful immutable historical artifacts. | none |
| #19 | Completed by INTERACTION-001A: explicit report-log edit mode, stable-ID draft, atomic commit, full cancel, and command/text separation. | none |
| #20 | Completed by INTERACTION-001B: exactly-once terminal restoration, runtime emergency checkpoint custody, primary-error preservation, and PTY certification. | none |
| #24 | Completed by INTERACTION-001C: explicit Bound/Unbound/Disabled state, declared contextual aliases, mandatory recovery-safe Ctrl-C, configurable F1, hidden-fallback removal, and atlas/palette/runtime parity. | none |
| #22 | Active draft versus category metadata remains a domain/UI distinction. | DOMAIN-002 |

## Immediate action

Implement RECONCILIATION-001B2C for issue #10:

1. define a stable clear-all/reset receipt that binds deleted idle history, cleared sediment, replacement active state, and affected daily contributions;
2. replay every separate publication idempotently and certify every durable kill point;
3. retain receipt evidence after any incomplete publication;
4. then reconcile the initial active-start/checkpoint window;
5. bind exact transition-edge sediment classification to the same authority;
6. expose checkpoint capture, recovery target, reconstructed duration, and deterministic cutoff policy in the recovery interface.

After issue #10 reaches evidence-based closure, return to the merge/reassignment transaction required to complete issue #13.
