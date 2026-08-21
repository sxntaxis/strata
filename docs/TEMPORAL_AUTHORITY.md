# Temporal authority

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

- each slice follows the canonical UTC interval and one operational-day window;
- whole-second allocation is computed cumulatively from the canonical session start, so sub-second timestamps crossing an exact day boundary cannot lose a second through independent flooring;
- slice seconds sum exactly to the canonical session's elapsed whole seconds;
- an endpoint exactly on a boundary creates no empty next-day slice;
- day, week, month, category-log, balance, and live-preview calculations consume slices rather than assigning the entire row to its end day;
- editing or deleting a session still targets one identity.

Imported records without absolute chronology are not promoted to live SQLite authority without explicit valid
provenance. Current SQLite session rows carry the full operational-day policy fields.

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
- strict persisted chronology validation without fabricated provenance;
- visible removal of the false sunrise mode;
- zero finish and repeated zero switch receipts without completed work rows;
- existing SQLite authority, lifecycle, recovery, configuration, and TUI process gates.
