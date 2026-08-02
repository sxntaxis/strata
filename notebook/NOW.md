---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-02
authority: working
summary: SEDIMENT-001D1 is complete; historical artifacts have explicit identity and immutable rendering.
next: Implement SEDIMENT-001D2 for issue #18: authoritative daily contributions, revision comparison, invalidation, and legacy disposition.
---

# NOW — Strata

## Current phase

The SQLite migration program, AUTHORITY-001, TEMPORAL-001, TEMPORAL-002, DOMAIN-001, REPORT-001, SEDIMENT-001A, SEDIMENT-001B, SEDIMENT-001C1, SEDIMENT-001C2, and SEDIMENT-001D1 are complete. Strata now has durable persistence, explicit time and classification authority, truthful report/export projections, conserved sediment mass and topology, bounded recovery, typed snapshot identity, and immutable historical viewing.

The project is completing **sediment conservation**.

## Accepted product baseline

- Strata is a continuous temporal ledger and an active timer.
- Time always passes; the baseline name is **idle**.
- Idle continues depositing sediment but is omitted from ordinary active-time accounting.
- Project and category are independent session axes.
- A CLI work session requires explicit category classification.
- Exact chronological history and accountable sedimentary history are both meaningful.
- Sediment is product function and artwork, not disposable decoration.
- One grain currently represents one elapsed second.
- Every due grain exists exactly once as placed or pending logical mass.
- Terminal dimensions are projection state, not canonical sediment dimensions.
- Pending mass may be compressed without changing count, identity, or FIFO category order.
- Runtime recovery may add missed mass but cannot replay unbounded physics or relax topology.
- Cumulative checkpoints, daily contributions, and derived previews are distinct artifacts.
- Historical viewing is immutable and cannot persist or advance a preview.

## Verified technical baseline

- SQLite schema version 5 is authoritative after explicit activation.
- CLI and TUI share repository, configuration, temporal, session-domain, and recovery boundaries.
- Reports use exact operational-day overlap slices and explicit provisional active state.
- JSON and ICS exports have deterministic ordering and authoritative UTC chronology.
- Sediment rendering emits one Braille character per drawable terminal cell.
- Blocked and detached elapsed mass remains category-preserving pending mass.
- Canonical coordinates and engine metadata survive terminal resize and recovery.
- `SandState` v2 stores ordered pending category/count runs and migrates v1 vectors.
- Billion-grain blocked or recovered intervals require one run rather than linear allocation.
- Runtime checkpoints cover autosave, detach, terminal closure, and crash recovery.
- Recovery publication is bounded, topology-preserving, atomic/reclaimable in SQLite, and deterministic in legacy-file authority.
- Snapshot envelopes record kind, day, source revision, provenance, idle policy, reconstruction status, and `SandState`.
- Legacy bare daily payloads are classified as cumulative legacy evidence.
- A cumulative artifact cannot satisfy a daily-contribution request.
- Missing or incompatible daily artifacts fall back to marked in-memory ledger-derived previews.
- Repeated historical rendering is deterministic and does not advance physics or mutate source state.
- Report UI exposes artifact kind, reconstruction status, and idle policy.
- Viewing or rebuilding a derived preview performs no persistence mutation.
- Persistence, temporal, projection, sediment, recovery, and snapshot failures remain fail-closed.

## Completed post-migration units

- **AUTHORITY-001** — issue #21: shared validated settings and fail-closed startup.
- **TEMPORAL-001** — issue #25: clock roles, discontinuity handling, fixed-offset civil authority, and reproducible history.
- **TEMPORAL-002** — issues #4, #23, #27: overlap allocation, false-sunrise removal, and zero-transition policy.
- **DOMAIN-001** — issues #2, #12: project/category identity, explicit classification, and idle vocabulary.
- **REPORT-001** — issues #1, #3, #14, #17, #28: truthful ranges/help, provisional projection, valid ICS, and deterministic ordering.
- **SEDIMENT-001A** — issues #16, #26: dimension truth, exact Braille width, complete ingress, and durable pending mass.
- **SEDIMENT-001B** — issue #7: canonical canvas and projection-only resize.
- **SEDIMENT-001C1** — issue #6 prerequisite: compressed runs and exact periodic arithmetic.
- **SEDIMENT-001C2** — issue #6: bounded topology-preserving recovery and protected evidence.
- **SEDIMENT-001D1** — issue #18 prerequisite: typed snapshot kinds, provenance, derived fallback, immutable viewing, and visible status.

Complete profile isolation remains open under issue #15. Project CRUD, TUI project selection, project-grouped reporting, and a TUI custom-range editor remain future work.

## Active sequence

1. **SEDIMENT-001D2** — issue #18: authoritative daily-contribution persistence, source-revision comparison, mutation invalidation, and legacy-row disposition.
2. **INTERACTION-001** — issues #19, #20, #24: explicit edit modes, terminal lifecycle guard, and truthful keybinding policy.
3. Reconcile partially satisfied issues #5, #10, #13 and later domain/profile work.

## Current risks

- Persisted daily rows still contain cumulative live state rather than typed daily contributions.
- Persisted daily artifacts are not yet compared against ledger-derived source revisions.
- Session edits/deletions do not yet invalidate every affected operational day under one explicit contract.
- Legacy cumulative daily rows do not yet have final archive/migration/removal disposition.
- Queued checkpoint mutations have no stable cross-authority receipt identity and fail closed.
- Interaction edit modes and terminal cleanup remain incompletely enforced.
- Complete profile switching/isolation remains open.

## Next

Implement **SEDIMENT-001D2** for issue #18. Persist typed daily contributions derived from canonical session slices, trust them only when source revisions match, rebuild all days affected by mutation, and disposition legacy cumulative daily rows without losing evidence or elevating derived previews into a competing authority.
