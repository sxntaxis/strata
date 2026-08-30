# Sediment authority

Status: implemented and certified
Program: SEDIMENT-002 + PLATEAU-001H hardening
Current published unit: PLATEAU-001H H4 grain-causal contact + SandState v5 real-profile green; H5 resize boundary release is native-certified and awaiting publication
Issues completed: #6, #7, #16, #18, #26
Last reviewed: 2026-08-28

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

Canvas growth never runs gravity or repacks existing grains. Pending logical mass may occupy newly available capacity through the normal pending-grain placement path. Live viewport widening is also a boundary event: when a visible left or right wall ceases to be a wall, resize may one-shot mark only the exact bottom-connected surface grain on that former wall as H4-mobilized, and only when the newly revealed outward diagonal creates dynamic relief `>1`. Resize itself still moves no grain, consumes no random choice, and changes no mass; any later spill is ordinary H4 gravity and slip-lineage behavior.

H5 resize-boundary release is native-certified at `3920ab3899f3249569f2dfb8c990e6389cb6fc47` (tree `8f3f69f7baeb15cd60423e5663b36647efd4a68b`). Focused H5 and H4/organic regressions, 272 library tests plus 23 integration tests, formatting, strict Clippy, help smoke, and diff hygiene passed. The only certification fallout was rustfmt-only in `src/sand/engine.rs`; H5 remains unmerged to main pending publication.

The current viewport is the active live-physics basin. New live grains enter at the visible top edge, gravity and diagonal movement remain within the visible rectangle, and the visible left and right edges act as temporary walls. Grains hidden by shrink remain frozen at their canonical coordinates. When re-expansion makes them visible, ordinary hidden grains reactivate normally and a former lateral wall may receive the one-shot boundary-release trigger above; narrowing never triggers it. Full clear is the one exception to monotonic canvas retention: it removes all placed and pending mass and resets the empty canonical canvas to the current viewport dimensions. Category-specific clearing, including Idle clear, preserves canonical extent.

## SandState persistence

`SandState` schema version 5 stores ordered pending runs, the optional canonical ingress focus, and exact mobilized grain
coordinates. Versions 1 through 4 remain readable; the v4 regional activity field is legacy migration input only.

- Version 1 `pending_grains` vectors migrate deterministically into adjacent runs.
- Version 2 compressed-pending states remain readable and upgrade to version 3 with no invented ingress focus.
- Older JSON with no pending or focus field loads those components empty/uninitialized.
- Empty pending collections and an uninitialized focus are omitted during serialization.
- The existing persisted RNG state seeds the first ingress focus after a v1/v2 migration; later v3 checkpoints preserve both RNG and focus for deterministic restart continuity.
- `SandState` stores canonical grid dimensions explicitly; recovery through a zero viewport restores them exactly, while an ordinary larger live viewport may monotonically expand the restored canvas.
- Ordinary restore normalizes unavailable category identities to idle; checkpoint recovery is stricter and refuses unavailable identities.
- v5 coordinates are canonical row-major, unique, in bounds, and must reference occupied cells; v5 rejects legacy active columns and pre-v5 states reject non-empty mobility.
- v5-to-v5 restore preserves exact mobility without normalization. v1-v4 migration validates and discards regional activity, then seeds only unsupported bottom-connected surface grains once without moving them.

Native v5 validation is complete at `f00b628bd37c42a9b27b2abb4b73b1068c74f551`. The exact snapshot/restore,
hidden-mobility resize/restart continuation, v4 migration, malformed-state fail-closed, recovery, recolor, and legacy
regression proofs pass. PR #89 published main `67ffd84d3c5c924211ac9a14b52b5749fb07ed8b`; the installed H4 binary is
SHA256 `b6f3af5247ce633b4c01c6232c1f1be057f7f9af562b6a5114f424b5f3559f93`. Real profile
`95446134-3681-4390-84d7-8d900ebbb892` completed the first v4→v5 owner smoke and second v5→v5 restart; persisted
SandState is v5 and sqlite-doctor passes. SQLite `user_version`, tables, and columns are unchanged; H4R2C runtime
behavior was not retuned.

## Metastable repose and local avalanches

H2 replaces the memoryless diagonal lottery with deterministic local repose. The canonical dot grid remains the geometry
authority. Supported height counts consecutive occupied cells upward from the visible bottom baseline and stops at the first
gap, so airborne grains do not inflate relief. Supported relief `<= 3` is statically stable; relief `> 3` starts a yield.
Active radius-one regions use dynamic relief `> 1`. One diagonal topple occurs per gravity pass, support-changing movement
refreshes local activity, and active straight-down free fall keeps activity alive. An active region settles only after a
pass has neither dynamic toppling nor active-region straight-down movement. Equal-relief side selection may consume RNG;
unstable-source selection follows deterministic persisted sweep order. This is a coarse supported-relief proxy, not a
literal pressure, force, or angle solver. H1 rain, pending mass, visible-basin custody, and HISTORY remain unchanged.

Native validation is complete at `f581de486a08547ea5fd74ef3ca2f2fb90e1eb34`. Real-cadence proof uses the product's 1000 ms ingress, 32 ms engine update, and every-second-update gravity ordering: 40x20 produced 449 avalanche events with median size 8, p95 82, and median quiet buildup 9 grains; 80x30 produced 658 events with median size 8, p95 20, and median quiet buildup 10 grains. Both runs conserved mass, produced no runaway event, and retained 0% one-move avalanche events. The former one-ingress-per-gravity harness is retained only as bounded overload stress because it feeds roughly 15.6 times faster than normal live cadence.

Daily-use H3 evidence adds one deliberately narrower shape invariant without retuning H2: a bottom-supported column whose two immediate visible neighbors both have supported height zero may stand two dots high, but a third supported dot is not allowed to remain as an isolated one-column needle. At height three or more, that source uses an effective static cap of two and yields through the ordinary H2 diagonal-topple/avalanche path. A peak with support on either immediate side is not an isolated spire and continues to use normal static relief `3`; dynamic relief remains `1`. This is geometry hardening only: no rain, RNG policy, avalanche state, persistence schema, resize custody, or HISTORY semantics change.

H3 native validation is complete at `VALIDATED_HEAD` `26fe55d`. The full suite passed with 272 library tests and 23 integration tests; focused boundary proofs cover isolated 2-dot stability, isolated 3-dot yield, one-sided/broad neighbor protection, visible-wall protection, mass conservation, and the settled-profile invariant. Corrected live cadence produced 446 events at 40x20 (median 8, p95 108, max 274, quiet buildup 9, 0% one-move) and 642 events at 80x30 (median 8, p95 24, max 52, quiet buildup 10, 0% one-move), with mass conserved, events of at least 10 moves, and no runaway. H3 adds no serialized field and requires no SandState or profile migration beyond existing v4.

H3 was subsequently published through main `f3590a7aeb69a4b88cef90862bb01eb7afd564ba` and installed on the real profile as binary SHA256 `fc6f806ba174313b9e89a7aa9814cf6ccf9e76a4ff017c755775a92421dd0350`. Daily-use evidence then rejected both two-dot and three-dot one-column prominences while preserving sloped walls and one-sided support, so H3 is historical evidence rather than the final formation rule.

H4 candidate authority replaces static relief/spire exceptions with contact support. An ordinary grain that reaches support may settle when the cell below plus at least one lower diagonal (or visible wall) is solid; an unsupported landing becomes an exact mobilized grain. Mobilized grains retain dynamic relief `1`, mobility travels with the exact grain through vertical/diagonal motion, real support loss wakes only exact dependents, and a diagonal topple may continue a cascade down the newly exposed same-column slip face only while that surface still has a dynamic route. Regional avalanche radii, proximity-based `active_vertical`, global static scans, and peak-height heuristics are absent.

The H4R2C behavior candidate is native-green at `579f3e1b652a2d90efcfcef65e1910d199e464ba`: 40x20 / 10,000 ingress produced 621 slip-lineage cascades including 99 multi-lineage episodes; 80x30 produced 1,070 cascades including 152 multi-lineage episodes. Both conserved 10,000/10,000 mass, completed without runaway or continuous motion, preserved `0/6/5`-like slopes and broad hills, and settled with interior one-column prominence at most one dot. SandState v5 persistence and the one-time pre-v5 semantic migration are now published and real-profile green. The retained pre-v5 backup and H3 rollback binary pair the forward boundary; the pre-v4 rollback pair remains retained.

## Organic live formation

Live formation remains an ordinary falling-sand process; SEDIMENT-002 changes local stochastic personality, not sediment meaning or mass authority.

For each visible grain during a gravity pass:

1. fall straight down whenever that cell is open;
2. if down is blocked, evaluate supported local repose and active-avalanche state;
3. static failure uses relief greater than three and active continuation uses relief greater than one;
4. after a deterministic yield, take the preferred reachable diagonal, randomizing only an equal-relief tie;
5. if neither diagonal is open, remain in place without consuming a friction/lateral choice.

The former one-quarter slide personality is retired by H2. No per-grain friction age, pressure, or slope field exists. The
existing alternating horizontal sweep remains an ordering/fairness mechanism.

New physical ingress keeps the rain-like cadence while introducing only a weak long-term spatial preference. One persisted **ingress focus** wanders slowly across the visible top edge, but it must not be visually traceable as a nozzle from a handful of falling grains. For each physical ingress:

1. initialize the focus from the persisted RNG when no focus exists;
2. otherwise move the focus by at most one canonical dot only on an occasional focus-update event, so it changes on a slower timescale than individual grains;
3. begin with a full-visible-width rain sample for every grain;
4. on only an occasional bias event, draw one second full-width candidate and prefer whichever candidate is closer to the focus, using RNG only for an equal-distance tie;
5. if the sampled top cell is occupied, choose the nearest currently free visible ingress column, randomizing only an equal-distance tie;
6. if no visible top column is free, leave the logical mass pending exactly as before.

The hardening personality keeps the one-in-four focus move and applies the soft two-candidate focus preference on one-in-four ingress samples. There is no hard local radius: every grain retains full-width support, and most grains are ordinary uniform rain. The intended visual test is temporal: short observation should read as rain, while accumulated mass over a much longer interval should reveal that one region received somewhat more sediment. These are private implementation constants; Settings do not expose them.

The ingress focus is small stochastic engine state, not grain provenance. Actual grain placement does not overwrite the focus when occupancy forces a farther fallback. The focus shifts by the same horizontal offset when the canonical canvas expands around its center. A shrink does not mutate hidden canonical topology; when the focus lies outside the smaller visible basin, the next physical ingress clamps it into that basin without placing outside it. Re-expansion preserves that focus/canonical relationship; any lateral boundary release is a separate H4 mobility trigger and does not move the focus. Full clear resets the focus along with the pile, while the RNG stream continues; category-specific clear leaves the focus intact.

No terrain generator, mountain template, post-settlement sculpting, temporal grain provenance, pressure solver, or user-facing randomness controls are part of this authority.

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

Balance historical background is visual memory, not a synthetic chart. While the live simulation crosses an operational-day cutoff, Strata captures the exact cumulative canonical `SandState` after processing events due through that boundary. For a fixed 06:00 day start, the artifact for a day is therefore the canonical canvas photo taken at the following 06:00 cutoff.

The day-end artifact is first-write-wins evidence:

- it preserves exact grain coordinates, category identity, pending mass, frame/sweep/RNG metadata, and canonical grid dimensions;
- later terminal resize, ledger reconciliation, report viewing, or category/session editing does not rewrite it;
- each operational day may therefore own a different canonical canvas size;
- `snapshot_kind = 'daily'` stores this cumulative visual checkpoint, while `daily-contribution` remains a separate accounting artifact.

If Strata did not observe a boundary through the ordinary live simulation path—for example because it was closed, detached through the cutoff, or bounded recovery deliberately skipped historical physics—it does not fabricate an authentic photo. Balance may then show a `DerivedPreview`, explicitly marked reconstructed.

## Immutable historical viewing

Historical viewing is projection-only:

- Balance prefers the authentic day-end checkpoint for the selected interval end day;
- the snapshot envelope and `SandState` remain immutable;
- rendering restores a clone into a fresh viewport engine;
- a smaller current viewport crops the historical canvas around horizontal center and bottom baseline;
- a larger current viewport expands only the temporary rendering clone, leaving the stored dimensions and topology unchanged;
- physics `update()` is never called;
- repeated rendering at the same viewport is deterministic;
- cache identity includes the serialized artifact and viewport;
- the report UI exposes kind, reconstruction status, and idle policy;
- viewing never writes or deletes persistence.

Day, week, and month Balance use the visual artifact for the selected interval's end day. The numerical report rows remain ledger-derived for the selected period. If no authentic photo exists, an in-memory `DerivedPreview` is the visual fallback and never becomes authority merely by being viewed.

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

## Historical correction and retained current sediment

Retroactive ledger correction does not replay historical physics and does not make the current pile a complete ledger chart. The live canonical `SandState` may have lost earlier mass through full clear or category-specific clear, while arbitrary historical assignment may also fill a true chronological gap that never emitted sand.

When a historical assignment changes already-classified canonical seconds from one category to another, Strata derives a category-transfer count from those changed seconds. The current pile then applies that transfer only against source-category mass that is still retained:

- placed source grains are recolored deterministically in canonical serialized order before pending mass;
- pending mass is recolored without changing FIFO order or total count;
- grain coordinates, topology, canvas dimensions, frame count, sweep direction, RNG state, and total logical mass do not change;
- true-gap seconds have no source transfer and therefore create no current grains;
- if prior clears leave fewer source-category grains than the transfer requests, only the retained amount is recolored; unrelated categories are never consumed to make the pile numerically match corrected ledger history;
- because grains intentionally carry no temporal/session provenance, the deterministic category-only choice does not claim that a particular recolored grain physically originated in the corrected interval.

The resulting current `SandState`, runtime checkpoint, canonical history rewrite, and affected `DailyContribution` rows publish coherently in one SQLite transaction. The application installs that exact committed `SandState` after the receipt returns. This operation adds no per-grain timestamp or session identity.

Authentic first-write day-end `daily` checkpoints remain immutable visual evidence and are never recolored by later history correction. Ledger-derived `DailyContribution` and in-memory `DerivedPreview` continue to reflect corrected chronology under their existing authority.

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

HISTORY-001E retained-current-sediment recolor is native-green at `d67c8e382708dbbf3f71bf2a67d7daa81b2e36b8` with 263 tests plus isolated runtime/restart proof. SEDIMENT-002 organic formation is implemented after that certified frontier and is awaiting native certification. The historical counts below remain the earlier SEDIMENT-001 baseline evidence.

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
