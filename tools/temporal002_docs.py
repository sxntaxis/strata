from pathlib import Path

# README
path = Path('README.md')
text = path.read_text()
text = text.replace(
"- **Historical report grouping** uses the operational-day key persisted with each completed session; later offset changes do not regroup old history.\n",
"- **Historical allocation** uses each completed session's persisted fixed-offset boundary policy and absolute interval; later setting changes do not redivide old history.\n",
)
text = text.replace(
"The current policy is a **fixed offset**, not an IANA timezone. It is deterministic across travel and seasonal clock changes but does not automatically apply daylight-saving transitions. Sunrise semantics remain separate work. The full contract is recorded in [`docs/TEMPORAL_AUTHORITY.md`](docs/TEMPORAL_AUTHORITY.md).\n",
"The current policy is a **fixed clock under a fixed UTC offset**, not an IANA timezone. It is deterministic across travel and seasonal clock changes but does not automatically apply daylight-saving transitions. The former `sunrise` option never performed solar calculation and has been removed; an existing `day_start_mode: \"sunrise\"` setting is rewritten visibly to `fixed` while preserving its configured hour and minute.\n\nA completed session remains one canonical ledger row. Reports project exact overlap slices at operational-day boundaries using the policy stored with that session, so a cross-boundary interval contributes only its overlapping seconds to each day without losing identity or creating empty exact-boundary fragments. Transitions whose whole-second duration is zero still complete or switch active state transactionally, but they do not create ordinary work rows. The full contract is recorded in [`docs/TEMPORAL_AUTHORITY.md`](docs/TEMPORAL_AUTHORITY.md).\n",
)
text = text.replace('  "day_start_mode": "sunrise",', '  "day_start_mode": "fixed",')
text = text.replace(
"- `day_start_mode` accepts `fixed` or `sunrise`.\n",
"- `day_start_mode` accepts only `fixed`. Existing `sunrise` values are migrated visibly to `fixed`; Strata never implemented solar sunrise calculation.\n",
)
text = text.replace(
"- `month` uses calendar months: current month-to-date, then complete prior calendar months.\n",
"- `month` uses calendar months: current month-to-date, then complete prior calendar months.\n- Day, week, and month totals allocate canonical sessions by exact overlap with their persisted operational-day boundary policy.\n- A zero-whole-second finish or switch is a transition event, not a completed work row.\n",
)
path.write_text(text)

# Temporal authority
Path('docs/TEMPORAL_AUTHORITY.md').write_text('''# Temporal authority

Status: accepted and certified
Implemented by: TEMPORAL-001 and TEMPORAL-002
Issues: #25, #4, #23, #27
Last reviewed: 2026-08-02

## Purpose

Strata must preserve interval meaning when wall clocks jump, processes restart, users travel, configuration changes, or work crosses an operational-day boundary. Clock authority, boundary policy, interval identity, and report allocation are therefore explicit and separate.

## Clock roles

| Question | Authority |
|---|---|
| How much time elapsed while the TUI process remained live? | `std::time::Instant` monotonic elapsed time |
| What absolute timestamps are persisted? | UTC |
| How are new civil clock labels rendered? | Validated configured fixed UTC offset |
| What policy divides a new session into operational days? | Fixed UTC offset plus fixed boundary minute captured with that session |
| How is historical duration assigned to a report day? | Exact overlap slices projected from the canonical interval using its persisted policy |
| How is elapsed time reconstructed after process death? | Checked UTC wall interval, because the previous monotonic clock is unavailable |

Machine-local timezone is not an authority in production temporal paths.

## Live reconciliation

A live session begins with both a UTC timestamp and a monotonic anchor. At finish or layer switch:

1. elapsed whole seconds come from the monotonic anchor;
2. Strata derives the expected UTC endpoint as `started_at_utc + monotonic_elapsed`;
3. that endpoint is compared with observed UTC wall time;
4. divergence of five seconds or less is ordinary scheduler/NTP jitter and the monotonic-derived endpoint is committed;
5. larger forward or backward divergence fails visibly before the transition consumes active state.

Strata does not clamp negative intervals, cast them to unsigned durations, or silently choose wall time over monotonic time.

## Restart and unattended recovery

A monotonic anchor cannot be serialized across process death. CLI stop, startup recovery, and checkpoint restoration therefore use a checked UTC wall interval.

- A start later than the observed end is rejected.
- An unattended interval of seven days or less can be reconstructed normally.
- A longer CLI interval requires `strata stop --accept-clock-jump`.
- The override accepts the recorded wall interval; it does not infer a correction.
- Reconstructing an `Instant` uses checked subtraction and fails visibly if the platform cannot represent it.

Historical catch-up mutations use their recorded UTC schedule rather than current wall time.

## Fixed-clock boundary policy

The supported civil policy is a fixed boundary clock under a fixed UTC offset. Each newly completed session stores:

- absolute UTC start and end;
- the fixed UTC offset used for civil interpretation;
- the boundary minute within that civil day.

This policy is deterministic under travel and later configuration changes. It deliberately does not implement IANA timezone rules, daylight-saving transitions, or solar calculation.

The former `sunrise` mode was only a label over the same fixed hour and minute. TEMPORAL-002 removes it from the domain and UI. When startup encounters `day_start_mode: "sunrise"`, Strata atomically rewrites it to `fixed`, preserves the configured hour and minute, and emits an explicit warning that solar sunrise calculation was never implemented.

## Canonical sessions and report slices

A logical session remains one canonical ledger identity. Crossing a boundary does not create multiple authoritative rows.

Reports instead project immutable `SessionSlice` values:

- each slice is the exact overlap between the canonical UTC interval and one operational-day window;
- slice seconds sum exactly to the canonical session's elapsed whole seconds;
- an endpoint exactly on a boundary creates no empty next-day slice;
- day, week, month, category-log, balance, and live-preview calculations consume slices rather than assigning the entire row to its end day;
- editing or deleting a session still targets one identity.

Older legacy rows without absolute chronology retain their persisted day and elapsed duration rather than receiving fabricated boundary provenance. New legacy CSV rows and deterministic SQLite bundles carry the full policy fields.

## Zero-duration transitions

Strata records work in whole seconds. A finish or switch with zero elapsed whole seconds is therefore a transition event, not completed work.

- no completed session row is inserted;
- no zero-duration report or sediment contribution is created;
- active finish/switch/reset state still changes transactionally;
- runtime receipts are still written and acknowledged, preserving idempotence and crash recovery;
- repeated rapid switches cannot accumulate phantom sessions.

SQLite schema constraints reject ordinary completed rows with non-positive duration while permitting receipt-only transitions.

## Failure matrix

| Condition | Behavior |
|---|---|
| Live wall clock differs from monotonic-derived end by more than five seconds | Block transition; preserve active state; show recovery |
| Live wall/monotonic difference is at most five seconds | Commit monotonic duration and derived UTC endpoint |
| Persisted start is in the future | Fail without consuming active state |
| Cross-process wall interval exceeds seven days | Require explicit `--accept-clock-jump` for CLI stop |
| Configured UTC offset or fixed boundary is invalid | Startup fails before authority resolution |
| Removed `sunrise` mode is encountered | Rewrite visibly to fixed-clock policy, preserving hour/minute |
| Session crosses one or more operational boundaries | Preserve one canonical row; reports allocate exact overlap slices |
| Session ends exactly on a boundary | Do not create an empty following-day slice |
| Finish or switch measures zero whole seconds | Commit transition/receipt without a work row |
| Historical configuration changes | Existing sessions retain their captured policy and allocation |

## Certification

TEMPORAL-001 and TEMPORAL-002 cover:

- future timestamps, backward/forward jumps, ordinary jitter, suspend-like agreement, and long-interval confirmation;
- fixed-offset behavior across seasonal dates and travel/configuration changes;
- cross-boundary conservation, exact-boundary behavior, and overlap-based repository ranges;
- deterministic bundle round-trip and SQLite schema upgrade;
- strict old/new legacy CSV compatibility without fabricated provenance;
- visible migration and UI removal of the false sunrise mode;
- zero finish and repeated zero switch receipts without completed work rows;
- existing SQLite authority, lifecycle, recovery, configuration, and TUI process gates.
''')

# Architecture
path = Path('docs/ARCHITECTURE.md')
text = path.read_text()
text = text.replace('Last reviewed: 2026-08-01', 'Last reviewed: 2026-08-02', 1)
text = text.replace(
"- `src/temporal.rs` — checked wall intervals, monotonic/wall reconciliation, fixed-offset civil projection, and operational-day allocation.\n",
"- `src/temporal.rs` — checked wall intervals, monotonic/wall reconciliation, fixed-clock civil policy, operational-day windows, and exact overlap slicing.\n",
)
anchor = "The detailed contract and failure matrix are `docs/TEMPORAL_AUTHORITY.md`.\n"
addition = """

TEMPORAL-002 completes the remaining interval semantics:

- one canonical session identity owns chronology, editing, deletion, and provenance;
- each new session captures its fixed UTC offset and fixed boundary minute;
- reports derive exact overlap slices instead of assigning an entire cross-boundary row to its ending day;
- exact-boundary endpoints create no empty fragments and allocated seconds are conserved;
- the false `sunrise` mode is removed and existing configuration is migrated visibly to fixed-clock policy;
- zero-whole-second finishes and switches create transactional receipts and state transitions but no completed work rows;
- SQLite schema version 5 and bundle schema version 2 preserve the new policy fields.
"""
if anchor not in text:
    raise SystemExit('architecture anchor missing')
text = text.replace(anchor, anchor + addition, 1)
text = text.replace(
"Persistence structure, startup configuration fallback, and clock authority are no longer the primary risks. The next program begins with remaining interval semantics:\n\n1. define overlap allocation, honest sunrise behavior, and zero-duration transitions;\n2. correct reporting, export, and classification semantics;\n3. establish a conserved sediment model independent of viewport and mutable previews.\n",
"Persistence structure, startup configuration fallback, clock authority, and interval-boundary semantics are no longer the primary risks. The next program is domain and projection correctness:\n\n1. reconcile project/classification and explicit idle semantics;\n2. correct reporting and export semantics;\n3. establish a conserved sediment model independent of viewport and mutable previews.\n",
)
path.write_text(text)

# Decisions
path = Path('docs/DECISIONS.md')
text = path.read_text().replace('Last reviewed: 2026-08-01', 'Last reviewed: 2026-08-02', 1)
anchor = '| STRATA-D014 | Live duration is monotonic; persisted timestamps are UTC; civil projection uses the validated fixed offset; persisted operational-day keys own historical grouping; ambiguous clock discontinuities fail closed. | implemented and certified |\n'
addition = (
'| STRATA-D015 | A logical session remains one canonical ledger identity; reports allocate its duration through exact operational-day overlap slices using policy captured with the session. | implemented and certified |\n'
'| STRATA-D016 | Fixed-clock policy is the only supported operational-day mode; the former sunrise label is removed and migrated visibly because no solar calculation existed. | implemented and certified |\n'
'| STRATA-D017 | Zero-whole-second finishes and switches are transactional transition events with receipts, not completed work rows or sediment. | implemented and certified |\n'
)
if anchor not in text:
    raise SystemExit('decision anchor missing')
text = text.replace(anchor, anchor + addition, 1)
path.write_text(text)

# NOW
Path('notebook/NOW.md').write_text('''---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-02
authority: working
summary: Persistence, configuration, clock authority, and interval-boundary semantics are complete; domain reconciliation now leads the frontier.
next: Implement DOMAIN-001 for issues #2 and #12 residuals: project/classification authority and explicit idle semantics.
---

# NOW — Strata

## Current phase

The SQLite migration program, AUTHORITY-001, TEMPORAL-001, and TEMPORAL-002 are complete. Strata now has durable authority, fail-closed configuration, explicit clock roles, reproducible fixed-clock boundaries, exact overlap allocation, and truthful zero-transition semantics.

The project is moving from **temporal foundations** to **domain and projection correctness**.

## Accepted product baseline

- Strata is a continuous temporal ledger and an active timer.
- Time always passes; the accepted baseline name is **idle**.
- Idle continues depositing sediment but is omitted from ordinary active-time accounting.
- Strata is general-purpose rather than freelancing-specific.
- Exact chronological history and accountable sedimentary history are both meaningful.
- Sediment is product function and artwork, not disposable decoration.
- One grain currently represents one elapsed second.
- Braille-cell color mixing is intentional.

## Verified technical baseline

- SQLite schema version 5 is authoritative after explicit activation.
- CLI and TUI share repository, runtime-coordination, configuration, and temporal boundaries.
- Live duration is monotonic; persisted absolute timestamps are UTC.
- Clock discontinuities fail closed; future starts and unsafe unattended intervals require explicit handling.
- The only supported operational-day policy is a fixed clock under a fixed UTC offset.
- New sessions capture offset and boundary-minute provenance.
- Canonical sessions remain single identities while reports allocate exact overlap slices across operational days.
- Exact-boundary endpoints create no empty fragments and allocated seconds are conserved.
- Existing `sunrise` configuration is migrated visibly to fixed policy; no solar behavior is claimed.
- Zero-whole-second finishes and switches retain transactional receipts but create no work rows.
- Old legacy rows remain readable without invented chronology; new CSV and bundle formats preserve temporal provenance.
- Persistence failure freezes mutation and offers retry, reload, emergency export, safe exit, or explicit unsafe exit.

## Completed post-migration units

- **AUTHORITY-001** — issue #21: shared validated settings and fail-closed CLI/TUI startup configuration.
- **TEMPORAL-001** — issue #25: clock roles, discontinuity handling, fixed-offset civil authority, and reproducible history.
- **TEMPORAL-002** — issues #4, #23, #27: overlap allocation, removal of false sunrise semantics, and zero-transition policy.

Complete profile isolation remains open under issue #15. IANA timezone/DST adoption remains a separate future decision.

## Active sequence

1. **DOMAIN-001** — issues #2 and #12 residuals: project/classification model and explicit idle semantics.
2. **REPORT-001** — issues #1, #14, #17, #28: custom ranges, provisional active time, valid ICS, deterministic ordering.
3. **SEDIMENT-001** — issues #6, #7, #16, #18, #26: conserved logical sediment independent of viewport and mutable previews.
4. **INTERACTION-001** — issues #19, #20, #24: explicit edit modes, terminal lifecycle guard, truthful keybinding policy.

## Current risks

- Project strings are preserved but do not yet have one complete product/domain authority.
- The accepted idle rename is not yet reflected consistently in runtime vocabulary and classification rules.
- Reports and exports still need custom ranges, provisional active-time policy, valid ICS, and deterministic ordering.
- Sediment rendering, resize, catch-up, and snapshots still lack one conservation model.

## Next

Implement **DOMAIN-001**. Reconcile issues #2 and #12 against current SQLite and temporal behavior before changing schema or terminology; preserve canonical interval identity and overlap semantics.
''')

# Issue reconciliation
path = Path('notebook/work/ISSUE-RECONCILIATION-001.md')
text = path.read_text().replace('updated: 2026-08-01', 'updated: 2026-08-02', 1)
text = text.replace(
'| #4, #23, #27 | Interval-boundary allocation, misleading sunrise semantics, and zero-duration policy are the next coupled temporal risks. | TEMPORAL-002 |',
'| #4, #23, #27 | Completed by TEMPORAL-002: canonical-session overlap allocation, visible removal/migration of false sunrise semantics, and receipt-only zero transitions. | none |',
)
text = text.replace(
'Implement TEMPORAL-002 for issues #4, #23, and #27. Clock authority is now explicit; the remaining question is how truthful intervals are divided at boundaries, named, and represented when their duration is zero.',
'Reconcile issues #2 and #12 under DOMAIN-001. SQLite preserves project strings and TEMPORAL-002 preserves truthful interval identity, but project/classification authority and explicit idle semantics remain unresolved.',
)
path.write_text(text)

# Reliability
path = Path('notebook/work/RELIABILITY-001-persistence-and-audit-remediation.md')
text = path.read_text().replace('updated: 2026-08-01', 'updated: 2026-08-02', 1)
anchor = 'Profile switching remains separate under issue #15.\n'
addition = '''

### TEMPORAL-002 — issues #4, #23, #27

- canonical sessions remain single ledger identities;
- reports allocate exact overlap slices using fixed-offset boundary provenance captured per session;
- cross-boundary seconds are conserved and exact-boundary endpoints create no empty fragments;
- the false sunrise mode is removed and existing configuration migrates visibly to fixed-clock policy;
- zero-whole-second finishes and switches create transactional receipts without completed work rows;
- SQLite schema version 5, bundle schema version 2, and backward-compatible legacy CSV preserve the policy.
'''
if anchor not in text:
    raise SystemExit('reliability anchor missing')
text = text.replace(anchor, anchor + addition, 1)
text = text.replace(
'1. **TEMPORAL-002** — issues #4, #23, #27.\n2. **DOMAIN-001** — issues #2 and #12 residuals.\n3. **REPORT-001** — issues #1, #14, #17, #28.\n4. **SEDIMENT-001** — issues #6, #7, #16, #18, #26.\n5. **INTERACTION-001** — issues #19, #20, #24.\n',
'1. **DOMAIN-001** — issues #2 and #12 residuals.\n2. **REPORT-001** — issues #1, #14, #17, #28.\n3. **SEDIMENT-001** — issues #6, #7, #16, #18, #26.\n4. **INTERACTION-001** — issues #19, #20, #24.\n',
)
path.write_text(text)
