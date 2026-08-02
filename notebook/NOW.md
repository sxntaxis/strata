---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-02
authority: working
summary: SEDIMENT-001A and 001B are complete; logical mass and topology now survive ingress blockage and terminal resize.
next: Implement SEDIMENT-001C for issue #6: bounded, retry-safe detached recovery.
---

# NOW — Strata

## Current phase

The SQLite migration program, AUTHORITY-001, TEMPORAL-001, TEMPORAL-002, DOMAIN-001, REPORT-001, SEDIMENT-001A, and SEDIMENT-001B are complete. Strata now has durable persistence, explicit clock and boundary semantics, preserved project/category identity, truthful deterministic report/export projections, lossless sediment ingress, and canonical sediment topology independent of terminal geometry.

The project is implementing **sediment conservation**.

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
- Every due grain exists exactly once as placed or pending logical mass.
- Terminal-cell dimensions and Braille-dot grid dimensions are separate units.
- The persisted logical grid owns canonical topology; terminal resize is projection only.

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
- Sediment rendering emits one Braille character per drawable terminal cell.
- Randomized ingress scans every physical column before declaring blockage.
- Fully blocked grains remain category-preserving pending mass and survive persistence/restore.
- `grain_count` accounts for placed plus pending logical mass.
- Terminal shrink crops presentation without deleting or repacking hidden grains.
- Terminal expansion pads presentation without stretching or relocating canonical grains.
- Resize leaves canonical dimensions, coordinates, category neighborhoods, pending order, frame count, sweep direction, and RNG state unchanged.
- Persisted sediment restores its canonical dimensions on any opening viewport.
- The destructive resize module and resize-triggered gravity path are removed.
- Persistence, temporal, projection, and sediment-state failures remain fail-closed and recoverable.

## Completed post-migration units

- **AUTHORITY-001** — issue #21: shared validated settings and fail-closed CLI/TUI startup configuration.
- **TEMPORAL-001** — issue #25: clock roles, discontinuity handling, fixed-offset civil authority, and reproducible history.
- **TEMPORAL-002** — issues #4, #23, #27: overlap allocation, removal of false sunrise semantics, and zero-transition policy.
- **DOMAIN-001** — issues #2, #12: persisted project identity, explicit category requirement, and completed idle vocabulary migration.
- **REPORT-001** — issues #1, #3, #14, #17, #28: truthful ranges/help, provisional active projection, valid ICS, and deterministic ordering.
- **SEDIMENT-001A** — issues #16, #26: dimension-unit truth, exact Braille output width, complete ingress scanning, and durable pending logical mass.
- **SEDIMENT-001B** — issue #7: canonical logical canvas, projection-only resize, exact topology preservation, and direct canonical restore.

Complete profile isolation remains open under issue #15. Project CRUD, TUI project selection, project-grouped reporting, and a TUI custom-range editor remain future work.

## Active sequence

1. **SEDIMENT-001C** — issue #6: bounded, retry-safe detached recovery.
2. **SEDIMENT-001D** — issue #18: explicit immutable historical snapshot kinds and provenance.
3. **INTERACTION-001** — issues #19, #20, #24: explicit edit modes, terminal lifecycle guard, truthful keybinding policy.
4. Reconcile remaining partially satisfied issues #5, #10, #13 and later domain/profile work.

## Current risks

- Detached catch-up still replays simulation work rather than applying bounded logical mass.
- Recovery evidence and sediment/session commit do not yet have one explicit retry-safe transaction contract.
- Historical snapshots do not yet have explicit kinds and immutable provenance.
- Interaction edit modes and terminal cleanup remain incompletely enforced.
- Complete profile switching/isolation remains open.

## Next

Implement **SEDIMENT-001C** for issue #6. Restore committed canonical topology directly, calculate detached elapsed contribution once, add the missed category mass through bounded work, and retain recovery evidence until sediment and session state commit together without duplication or loss across repeated reopen.
