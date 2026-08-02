---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-02
authority: working
summary: Persistence, temporal semantics, and session classification are complete; report and export correctness now lead the frontier.
next: Implement REPORT-001 for issues #1, #14, #17, and #28: custom ranges, provisional active time, valid ICS, and deterministic ordering.
---

# NOW — Strata

## Current phase

The SQLite migration program, AUTHORITY-001, TEMPORAL-001, TEMPORAL-002, and DOMAIN-001 are complete. Strata now has durable authority, explicit clock and boundary semantics, canonical interval allocation, preserved project identity, and explicit activity classification.

The project is moving from **domain foundations** to **projection correctness**.

## Accepted product baseline

- Strata is a continuous temporal ledger and an active timer.
- Time always passes; the baseline name is **idle**.
- Idle continues depositing sediment but is omitted from ordinary active-time accounting.
- Project and category are independent session axes.
- A CLI work session requires explicit category classification.
- Strata is general-purpose rather than freelancing-specific.
- Exact chronological history and accountable sedimentary history are both meaningful.
- Sediment is product function and artwork, not disposable decoration.
- One grain currently represents one elapsed second.
- Braille-cell color mixing is intentional.

## Verified technical baseline

- SQLite schema version 5 is authoritative after explicit activation.
- CLI and TUI share repository, runtime-coordination, configuration, temporal, and session-domain boundaries.
- Canonical sessions preserve project, category, chronology, and boundary provenance independently.
- CLI omission of `--category` fails before active-state mutation under both authority phases.
- Explicit `idle` selects category ID `0`; `none` and `drift` remain compatibility aliases only.
- Idle is excluded from ordinary work totals while remaining part of sediment history.
- Legacy start/stop/reload and SQLite start/stop preserve the exact project string.
- TUI synchronization and emergency custody export do not erase project identity.
- JSON and ICS consume the persisted project.
- Legacy 8- and 12-column CSV remains readable; new 13-column rows preserve project.
- Canonical sessions remain single identities while reports allocate exact overlap slices across operational days.
- Persistence and temporal failures remain fail-closed and recoverable.

## Completed post-migration units

- **AUTHORITY-001** — issue #21: shared validated settings and fail-closed CLI/TUI startup configuration.
- **TEMPORAL-001** — issue #25: clock roles, discontinuity handling, fixed-offset civil authority, and reproducible history.
- **TEMPORAL-002** — issues #4, #23, #27: overlap allocation, removal of false sunrise semantics, and zero-transition policy.
- **DOMAIN-001** — issues #2, #12: persisted project identity, explicit category requirement, and completed idle vocabulary migration.

Complete profile isolation remains open under issue #15. Project CRUD, TUI project selection, and project-grouped reporting are not implied by DOMAIN-001.

## Active sequence

1. **REPORT-001** — issues #1, #14, #17, #28: custom ranges, provisional active time, valid ICS, deterministic ordering.
2. **SEDIMENT-001** — issues #6, #7, #16, #18, #26: conserved logical sediment independent of viewport and mutable previews.
3. **INTERACTION-001** — issues #19, #20, #24: explicit edit modes, terminal lifecycle guard, truthful keybinding policy.

## Current risks

- Reports lack the complete custom-range and provisional-active-time contract.
- ICS output and export ordering still need standards-correct, deterministic behavior.
- Sediment rendering, resize, catch-up, and snapshots still lack one conservation model.
- Project-oriented views and filters remain future product work, though identity is now preserved.

## Next

Implement **REPORT-001**. Reconcile issues #1, #14, #17, and #28 against canonical overlap slices and persisted project/category identity; do not change ledger truth to simplify an output format.
