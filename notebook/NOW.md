---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-02
authority: working
summary: SEDIMENT-001C2 is complete; sediment mass, topology, viewport behavior, and runtime recovery are conserved.
next: Implement SEDIMENT-001D for issue #18: explicit immutable snapshot kinds and provenance.
---

# NOW — Strata

## Current phase

The SQLite migration program, AUTHORITY-001, TEMPORAL-001, TEMPORAL-002, DOMAIN-001, REPORT-001, SEDIMENT-001A, SEDIMENT-001B, SEDIMENT-001C1, and SEDIMENT-001C2 are complete. Strata now has durable persistence, explicit clock and boundary semantics, preserved project/category identity, truthful report/export projections, lossless sediment ingress, viewport-independent topology, compressed logical mass, and bounded retry-safe runtime recovery.

The project is completing **sediment conservation**.

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
- Pending logical mass may be compressed into ordered category/count runs without changing count, identity, or FIFO category order.
- Runtime recovery may add missed logical mass but cannot replay unbounded physics or relax checkpoint topology.

## Verified technical baseline

- SQLite schema version 5 is authoritative after explicit activation.
- CLI and TUI share repository, runtime-coordination, configuration, temporal, and session-domain boundaries.
- Canonical sessions remain single identities while reports allocate exact overlap slices across operational days.
- Project and category survive legacy/SQLite lifecycle, TUI synchronization, custody export, JSON, and ICS.
- Reports accept inclusive custom operational-day ranges.
- Reports and general exports include active time by default as explicit provisional state; `--completed-only` selects committed history.
- Report and export ordering has deterministic tie-breakers.
- JSON schema version 2 carries stable UIDs, provisional state, and UTC endpoints.
- ICS uses authoritative UTC chronology, stable UIDs, RFC 5545-safe serialization, and independent parser certification.
- Idle is excluded from ordinary active-time totals and ICS work events while remaining part of sediment history.
- Sediment rendering emits one Braille character per drawable terminal cell.
- Randomized live ingress examines available physical columns before blockage.
- Fully blocked grains remain category-preserving pending mass and survive persistence/restore.
- `grain_count` accounts for placed plus pending logical mass.
- Terminal resize leaves canonical dimensions, coordinates, category neighborhoods, pending order, frame count, sweep direction, and RNG state unchanged.
- `SandState` schema version 2 stores ordered pending category/count runs and migrates version 1 vectors.
- A billion blocked or recovered grains require one run rather than a billion allocations.
- Exact periodic event counts and accumulator remainders are calculated without iterative replay.
- Runtime checkpoints cover autosave, detach, terminal closure, and crash recovery.
- Bounded recovery restores canonical topology and appends detached mass as compressed pending runs.
- Missed physics frames are not replayed and no relaxed catch-up topology is installed.
- SQLite recovery publication is atomic and committed evidence remains reclaimable until replaced by a fresh checkpoint.
- Legacy-file recovery uses deterministic target and committed markers for overwrite-safe retry.
- Normal shutdown retires pending or committed checkpoint evidence but protects recovering or quarantined evidence.
- Checkpoints with queued mutations fail closed because stable cross-authority mutation receipts are not defined.
- Persistence, temporal, projection, sediment, and recovery failures remain fail-closed and evidence-preserving.

## Completed post-migration units

- **AUTHORITY-001** — issue #21: shared validated settings and fail-closed CLI/TUI startup configuration.
- **TEMPORAL-001** — issue #25: clock roles, discontinuity handling, fixed-offset civil authority, and reproducible history.
- **TEMPORAL-002** — issues #4, #23, #27: overlap allocation, removal of false sunrise semantics, and zero-transition policy.
- **DOMAIN-001** — issues #2, #12: persisted project identity, explicit category requirement, and completed idle vocabulary migration.
- **REPORT-001** — issues #1, #3, #14, #17, #28: truthful ranges/help, provisional active projection, valid ICS, and deterministic ordering.
- **SEDIMENT-001A** — issues #16, #26: dimension truth, exact Braille output width, complete ingress scanning, and durable pending mass.
- **SEDIMENT-001B** — issue #7: canonical logical canvas, projection-only resize, exact topology preservation, and direct canonical restore.
- **SEDIMENT-001C1** — issue #6 prerequisite: compressed pending runs, `SandState` v2 migration, bounded bulk addition, and exact periodic arithmetic.
- **SEDIMENT-001C2** — issue #6: runtime checkpoints, bounded topology-preserving recovery, atomic/reclaimable evidence, and safe shutdown retirement.

Complete profile isolation remains open under issue #15. Project CRUD, TUI project selection, project-grouped reporting, and a TUI custom-range editor remain future work.

## Active sequence

1. **SEDIMENT-001D** — issue #18: explicit immutable historical snapshot kinds and provenance.
2. **INTERACTION-001** — issues #19, #20, #24: explicit edit modes, terminal lifecycle guard, truthful keybinding policy.
3. Reconcile remaining partially satisfied issues #5, #10, #13 and later domain/profile work.

## Current risks

- Historical sediment artifacts do not yet declare whether they are cumulative checkpoints, daily contributions, or derived previews.
- Historical viewing and session mutation do not yet have one explicit invalidation/provenance contract.
- Queued checkpoint mutations have no stable cross-authority receipt identity and therefore fail closed.
- Interaction edit modes and terminal cleanup remain incompletely enforced.
- Complete profile switching/isolation remains open.

## Next

Implement **SEDIMENT-001D** for issue #18. Define immutable snapshot kinds and source provenance, separate persisted historical artifacts from derived previews, specify mutation invalidation/rebuild rules, and make idle inclusion and reconstruction status explicit without creating a competing sediment authority.
