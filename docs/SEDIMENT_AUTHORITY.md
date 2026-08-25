# Sediment authority

Status: implemented and certified
Program: SEDIMENT-001
Current completed unit: SEDIMENT-001D2
Issues completed: #6, #7, #16, #18, #26
Last reviewed: 2026-08-02

## Purpose

Sediment is accountable visual history derived from elapsed time. Chronological sessions remain the exact time authority; sediment preserves explicit mass, category, topology, recovery, snapshot, and projection obligations.

## Logical mass

Every due grain exists exactly once as either:

- a placed grain in the canonical logical dot grid; or
- a pending grain waiting for ordinary live ingress.

Physical blockage never authorizes loss. `grain_count` represents placed plus pending mass. Pending grains retain category identity and FIFO category order.

Pending mass is stored as ordered category/count runs. Adjacent additions for the same category merge, while category transitions remain ordered. Bulk addition and persistence are independent of represented count, apart from bounded placement into currently free ingress columns. Count overflow fails visibly.

Clearing all sediment clears placed and pending mass. Category clearing and counted removal apply to both forms.

## Geometry and canonical topology

Terminal-cell dimensions and Braille-dot dimensions are distinct units:

- `cell_width` and `cell_height` are viewport dimensions;
- `grid_width_dots` and `grid_height_dots` are canonical logical-canvas dimensions;
- one terminal cell projects `dot_width × dot_height` logical dots.

The persisted logical grid owns coordinates, neighborhoods, category composition, and topology. Shrinking the terminal changes viewport state only and crops presentation without deleting hidden grains. Growing beyond the logical canvas expands it monotonically: old cells are copied around the horizontal center and bottom baseline, new cells begin empty, and the canvas is never shrunk again merely because the viewport shrinks.

Canvas growth never runs gravity or repacks existing grains. Pending logical mass may occupy newly available capacity through the normal pending-grain placement path. Once a maximum extent has been reached, shrink/grow oscillation within that extent is idempotent at the `SandState` level.

The current viewport is the active live-physics basin. New live grains enter at the visible top edge, gravity and diagonal movement remain within the visible rectangle, and the visible left and right edges act as temporary walls. Grains hidden by shrink remain frozen at their canonical coordinates and become active again when re-expansion makes them visible. Full clear is the one exception to monotonic canvas retention: it removes all placed and pending mass and resets the empty canonical canvas to the current viewport dimensions. Category-specific clearing, including Idle clear, preserves canonical extent.

## SandState persistence

`SandState` schema version 2 stores ordered pending runs.

- Version 1 `pending_grains` vectors migrate deterministically into adjacent runs.
- Older JSON with no pending field loads as an empty reservoir.
- Empty pending collections are omitted during serialization.
- `SandState` stores canonical grid dimensions explicitly; recovery through a zero viewport restores them exactly, while an ordinary larger live viewport may monotonically expand the restored canvas.
- Ordinary restore normalizes unavailable category identities to idle; checkpoint recovery is stricter and refuses unavailable identities.

## Bounded runtime recovery

Runtime checkpoints cover periodic autosave, detach, terminal closure, and crash recovery. They preserve canonical `SandState`, active classification, active-session start UTC, simulation UTC, periodic accumulator remainders, and one recovery target UTC.

Recovery follows:

1. claim and validate evidence;
2. persist a fixed recovery target;
3. restore checkpoint topology and engine metadata directly;
4. calculate due mass and remainders with checked integer arithmetic;
5. append missed mass as compressed pending runs;
6. publish recovered authority;
7. retain or replace checkpoint evidence according to commit state.

Missed physics frames are counted but never replayed. Recovery never installs a relaxed replacement topology. Work is independent of detached duration apart from validation and compact run changes.

The same bounded settlement primitive is used for long live catch-up. Backlog of eight seconds or less may use the short accelerated visual path; backlog beyond eight seconds settles directly to the current UTC boundary with checked periodic arithmetic instead of replaying physics frames. A user mutation during catch-up first settles to that mutation's exact UTC timestamp and then applies immediately, so current runtime no longer needs an in-memory queued-mutation path merely to wait for visual catch-up.

SQLite publishes recovered canonical sediment, active-session continuity, the current typed daily contribution, and checkpoint state atomically. Committed evidence remains reclaimable until a fresh pending checkpoint replaces it. After successful recovery, every operational day touched by canonical session slices is reconciled.

Recovery checkpoints, targets, and committed markers are SQLite-owned. Portable exports contain projections and
are not runtime recovery authority.

Normal shutdown may retire pending or committed evidence. Recovering or quarantined evidence remains protected. Current runtime does not create queued-mutation checkpoints. Old mutation-bearing checkpoints still fail closed because no stable cross-authority mutation receipt exists.

## Snapshot identity

A `SedimentSnapshot` envelope records:

- semantic kind;
- optional operational day;
- source revision;
- provenance;
- idle-inclusion policy;
- reconstruction status;
- canonical `SandState`.

Accepted kinds are:

- `CumulativeCheckpoint` — authentic canonical sediment at a capture point;
- `DailyContribution` — mass attributed to exactly one operational day;
- `DerivedPreview` — deterministic ledger reconstruction for viewing only.

These kinds are not interchangeable. Historical bare daily payloads are cumulative artifacts, not daily contributions.

## Authentic day-end visual memory

Karma historical background is visual memory, not a synthetic chart. While the live simulation crosses an operational-day cutoff, Strata captures the exact cumulative canonical `SandState` after processing events due through that boundary. For a fixed 06:00 day start, the artifact for a day is therefore the canonical canvas photo taken at the following 06:00 cutoff.

The day-end artifact is first-write-wins evidence:

- it preserves exact grain coordinates, category identity, pending mass, frame/sweep/RNG metadata, and canonical grid dimensions;
- later terminal resize, ledger reconciliation, report viewing, or category/session editing does not rewrite it;
- each operational day may therefore own a different canonical canvas size;
- `snapshot_kind = 'daily'` stores this cumulative visual checkpoint, while `daily-contribution` remains a separate accounting artifact.

If Strata did not observe a boundary through the ordinary live simulation path—for example because it was closed, detached through the cutoff, or bounded recovery deliberately skipped historical physics—it does not fabricate an authentic photo. Karma may then show a `DerivedPreview`, explicitly marked reconstructed.

## Immutable historical viewing

Historical viewing is projection-only:

- Karma prefers the authentic day-end checkpoint for the selected interval end day;
- the snapshot envelope and `SandState` remain immutable;
- rendering restores a clone into a fresh viewport engine;
- a smaller current viewport crops the historical canvas around horizontal center and bottom baseline;
- a larger current viewport expands only the temporary rendering clone, leaving the stored dimensions and topology unchanged;
- physics `update()` is never called;
- repeated rendering at the same viewport is deterministic;
- cache identity includes the serialized artifact and viewport;
- the report UI exposes kind, reconstruction status, and idle policy;
- viewing never writes or deletes persistence.

Day, week, and month Karma use the visual artifact for the selected interval's end day. The numerical report rows remain ledger-derived for the selected period. If no authentic photo exists, an in-memory `DerivedPreview` is the visual fallback and never becomes authority merely by being viewed.

## Authoritative daily contributions

`DailyContribution` is accounting evidence, not a historical canvas. It is derived from exact operational-day session slices, including the active provisional slice when applicable.

The builder:

- includes idle explicitly and deterministically;
- orders slices by chronology and stable session identity;
- conserves every represented second as compressed ordered pending runs;
- records `SessionLedger` provenance and reconstruction status;
- is independent of terminal and canonical-canvas dimensions;
- calculates a source revision from day, quantum, idle policy, category identity, elapsed seconds, slice endpoints, and session identity.

Description text and canvas dimensions are deliberately absent from the contribution revision because neither changes sediment mass or chronology. Consequently, resizing or clearing the visual canvas today cannot make an old accounting contribution stale. Persisted contribution reconciliation remains ledger-driven and separate from the immutable day-end visual artifact.

## Mutation and recovery reconciliation

Daily contribution reconciliation occurs at autosave, full-state flush, checkpoint recovery completion, and relevant session mutation boundaries.

- Deleting a canonical session captures every operational day touched by its exact overlap slices and reconciles each day after deletion.
- Description-only edits leave source revision unchanged and do not trigger sediment invalidation.
- Future category, chronology, or duration mutation must reconcile every before/after affected operational day.
- Recovery completion reconciles all days represented by completed and active canonical slices, including multi-day detached intervals.

## Historical evidence disposition

The current SQLite schema already distinguishes `daily` from `daily-contribution`. `daily` is cumulative visual evidence; `daily-contribution` is ledger-derived accounting evidence. New authentic day-end captures use `daily` with a typed `CumulativeCheckpoint` envelope and are never overwritten by later reconciliation.

A historical bare `daily` payload that is a valid `SandState` remains cumulative visual evidence and is wrapped as `LegacyDailyRow` when viewed. It is never reinterpreted as a daily contribution. Portable exports preserve both artifact classes but are not runtime authority.

## Certification

The authentic day-end visual-memory correction described above is implemented after the last certified baseline and is awaiting native certification. The counts below describe the previously certified SEDIMENT-001 baseline.

SEDIMENT-001 is certified through PRs #50–#55.

The final D2 implementation and multi-day recovery correction passed:

- formatting;
- strict Clippy with all targets/features and warnings denied;
- 161 unit tests;
- 9 CLI lifecycle tests;
- 6 configuration-authority tests;
- 1 report-help regression test;
- 12 SQLite/TUI process tests;
- 2 temporal-authority tests;
- doc tests.

Focused proofs cover compressed billion-grain mass, immutable rendering, revision reuse and stale fallback, mass conservation beyond physical capacity, typed SQLite round-trip, fault rollback, cross-day session deletion reconciliation, and multi-day recovery reconciliation.

## Remaining non-authority

SEDIMENT-001 is complete. The following remain separate future design questions rather than sediment defects:

- zoom, compression, panning, minimaps, or explicit canonical-canvas migration;
- final vertical chronology semantics beyond the accepted bottom-aligned viewport projection;
- safe queued-mutation checkpoint replay if stable cross-authority receipts are later defined;
- configurable temporal quantum and its migration rules.
