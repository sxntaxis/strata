---
id: ISSUE-RECONCILIATION-001
kind: work
state: active
authority: working
created: 2026-08-01
updated: 2026-08-03
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
| #10 | Completed by RECONCILIATION-001B1, B2A, B2B, B2C, B3A, B3B, and B3C: active/checkpoint generations, prepared legacy replay, non-destructive clear-all, atomic initial bootstrap, exact transition-edge sediment, persisted cutoff reuse, visible recovery classification, repeated restart proof, and schema-3 export parity are certified. | none |
| #13 | Partially completed by RECONCILIATION-001A and RECONCILIATION-001C1. Historical meaning survives archive/restore, and SQLite now has a complete revision-bound preview, atomic merge or zero-reference deletion, auditable receipts, retired-ID custody, portable bundle parity, and doctor integrity. Remaining scope is the legacy-file prepared receipt/replay protocol and explicit TUI review/confirmation. | RECONCILIATION-001C2 |
| #6, #7, #16, #18, #26 | Completed by SEDIMENT-001: conserved mass/topology/recovery and truthful immutable historical artifacts. | none |
| #19 | Completed by INTERACTION-001A: explicit report-log edit mode, stable-ID draft, atomic commit, full cancel, and command/text separation. | none |
| #20 | Completed by INTERACTION-001B: exactly-once terminal restoration, runtime emergency checkpoint custody, primary-error preservation, and PTY certification. | none |
| #24 | Completed by INTERACTION-001C: explicit Bound/Unbound/Disabled state, declared contextual aliases, mandatory recovery-safe Ctrl-C, configurable F1, hidden-fallback removal, and atlas/palette/runtime parity. | none |
| #22 | Active draft versus category metadata remains a domain/UI distinction. | DOMAIN-002 |

## Immediate action

Issue #10 is evidence-backed and complete. RECONCILIATION-001C1 has certified the SQLite lifecycle half of issue #13. Next:

1. design a prepared legacy-file lifecycle receipt that binds the C1 source/target metadata, complete counts, deterministic revision, transformed payloads, affected days, and retired-ID result;
2. publish and replay catalog, session, tag, canonical-sand, daily-artifact, detached-checkpoint, and lifecycle-custody effects idempotently from every durable kill point;
3. expose one explicit TUI preview and confirmation flow that recomputes the revision before mutation and keeps archive as the ordinary retirement path;
4. close issue #13 only after both authorities and the visible interaction are evidence-backed.

Do not treat archive as deletion, reuse a retired identity, or invent reassignment for unresolved references.
