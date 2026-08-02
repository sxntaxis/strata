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
| #10 | Partially completed by RECONCILIATION-001B1 and B2A: SQLite active/checkpoint generations are transactional; incompatible evidence blocks transitions; startup validates checkpoint identity; semantic-edge refresh is immediate; legacy switches now publish a prepared schema-3 receipt and replay session/catalog effects idempotently from every durable kill point. Remaining scope is legacy reset/finish receipts, initial active-start/checkpoint coherence, exact transition-edge sediment reconciliation, and explicit recovery-cutoff/uncertainty presentation. | RECONCILIATION-001B2B |
| #13 | Historical data-loss defect completed by RECONCILIATION-001A: active/archived metadata, reports, sand, tags, restore, and migration retain stable meaning under SQLite and legacy authority. Remaining scope is explicit merge/reassignment plus permanent-deletion policy and tests. | DOMAIN-002 or dedicated category-merge unit |
| #6, #7, #16, #18, #26 | Completed by SEDIMENT-001: conserved mass/topology/recovery and truthful immutable historical artifacts. | none |
| #19 | Completed by INTERACTION-001A: explicit report-log edit mode, stable-ID draft, atomic commit, full cancel, and command/text separation. | none |
| #20 | Completed by INTERACTION-001B: exactly-once terminal restoration, runtime emergency checkpoint custody, primary-error preservation, and PTY certification. | none |
| #24 | Completed by INTERACTION-001C: explicit Bound/Unbound/Disabled state, declared contextual aliases, mandatory recovery-safe Ctrl-C, configurable F1, hidden-fallback removal, and atlas/palette/runtime parity. | none |
| #22 | Active draft versus category metadata remains a domain/UI distinction. | DOMAIN-002 |

## Immediate action

Implement RECONCILIATION-001B2B for issue #10:

1. define stable legacy reset and finish receipts without weakening the accepted switch contract;
2. replay those transitions idempotently across every separate authority publication;
3. certify process death between each publication point;
4. reconcile the initial active-start/checkpoint window;
5. bind transition-edge sediment contribution to the same authoritative boundary;
6. expose checkpoint capture, recovery target, reconstructed duration, and deterministic cutoff policy in the recovery interface.

After issue #10 reaches evidence-based closure, return to the merge/reassignment transaction required to complete issue #13.
