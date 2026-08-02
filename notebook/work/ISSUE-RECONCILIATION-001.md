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
| #1, #3, #14, #17, #28 | Completed by REPORT-001: inclusive operational-day ranges, truthful calendar help, provisional active projection, valid ICS, and deterministic ordering. | none |
| #5, #10, #13 | SQLite integrity, active authority, and category archival likely satisfy substantial portions; verify every criterion before closing or rewriting. | reconciliation audit |
| #6 | Completed by SEDIMENT-001C1/C2: compressed pending mass, bounded topology-preserving runtime recovery, atomic/reclaimable checkpoint evidence, and safe shutdown retirement. | none |
| #7 | Completed by SEDIMENT-001B: canonical logical canvas and projection-only terminal resize. | none |
| #16, #26 | Completed by SEDIMENT-001A: lossless ingress, explicit geometry units, and exact Braille viewport dimensions. | none |
| #18 | Historical snapshot identity, provenance, immutability, and invalidation remain open. | SEDIMENT-001D |
| #19, #20, #24 | Interaction modes, terminal cleanup, and keymap truth remain independent of SQLite. | INTERACTION-001 |
| #22 | Active draft versus category metadata remains a domain/UI distinction. | DOMAIN-002 |

## Immediate action

Implement SEDIMENT-001D for issue #18. Mass, topology, viewport behavior, and runtime recovery are now conserved; historical sediment artifacts must declare their kind and provenance and remain immutable while viewed.
