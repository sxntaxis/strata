---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-02
authority: working
summary: SEDIMENT-001 is active; dimension truth and lossless ingress are the first conservation edge.
next: Complete SEDIMENT-001A for issues #16 and #26, then continue to viewport-independent logical sediment.
---

# NOW — Strata

## Current phase

The SQLite migration program, AUTHORITY-001, TEMPORAL-001, TEMPORAL-002, DOMAIN-001, and REPORT-001 are complete. Strata now has durable persistence, explicit clock and boundary semantics, preserved project/category identity, and truthful deterministic report/export projections.

The project is moving from **projection correctness** to **sediment conservation**.

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
- Canonical sessions remain single identities while reports allocate exact overlap slices across operational days.
- Project and category survive legacy/SQLite lifecycle, TUI synchronization, custody export, JSON, and ICS.
- Reports accept inclusive custom operational-day ranges.
- Reports and general exports include active time by default as explicit provisional state; `--completed-only` selects committed history.
- Report and export ordering has deterministic tie-breakers.
- JSON schema version 2 carries stable UIDs, provisional state, and UTC endpoints.
- ICS uses authoritative UTC chronology, stable UIDs, RFC 5545-safe text serialization, and independent parser certification.
- Idle is excluded from ordinary active-time totals and ICS work events while remaining part of sediment history.
- Persistence, temporal, and projection failures remain fail-closed and recoverable.

## Completed post-migration units

- **AUTHORITY-001** — issue #21: shared validated settings and fail-closed CLI/TUI startup configuration.
- **TEMPORAL-001** — issue #25: clock roles, discontinuity handling, fixed-offset civil authority, and reproducible history.
- **TEMPORAL-002** — issues #4, #23, #27: overlap allocation, removal of false sunrise semantics, and zero-transition policy.
- **DOMAIN-001** — issues #2, #12: persisted project identity, explicit category requirement, and completed idle vocabulary migration.
- **REPORT-001** — issues #1, #3, #14, #17, #28: truthful ranges/help, provisional active projection, valid ICS, and deterministic ordering.

Complete profile isolation remains open under issue #15. Project CRUD, TUI project selection, project-grouped reporting, and a TUI custom-range editor remain future work.

## Active sequence

1. **SEDIMENT-001** — issues #6, #7, #16, #18, #26: conserved logical sediment independent of viewport and mutable previews.
2. **INTERACTION-001** — issues #19, #20, #24: explicit edit modes, terminal lifecycle guard, truthful keybinding policy.
3. Reconcile remaining partially satisfied issues #5, #10, #13 and later domain/profile work.

## Current risks

- Sediment rendering, resize, catch-up, and snapshots still lack one conservation model.
- The relationship between logical sediment mass, viewport capacity, and historical topology is not yet explicit.
- Interaction edit modes and terminal cleanup remain incompletely enforced.
- Complete profile switching/isolation remains open.

## Next

Complete **SEDIMENT-001A**. Establish explicit terminal-cell versus dot-grid dimensions and retain every blocked due grain as pending logical mass. Then continue to SEDIMENT-001B without treating the current viewport as canonical storage.
