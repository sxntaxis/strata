# Temporal authority

Status: accepted and certified
Implemented by: TEMPORAL-001
Issue: #25
Last reviewed: 2026-08-01

## Purpose

Strata must preserve interval meaning when wall clocks jump, processes restart, users travel, or configuration changes. No single clock can answer every temporal question, so authority is divided explicitly by responsibility.

## Clock roles

| Question | Authority |
|---|---|
| How much time elapsed while the TUI process remained live? | `std::time::Instant` monotonic elapsed time |
| What absolute timestamps are persisted? | UTC |
| How are new start/end clock labels rendered? | Validated configured fixed UTC offset |
| Which operational day receives a newly completed session? | UTC endpoint projected through that fixed offset and configured cutoff |
| Which day contains an existing historical session? | The operational-day key persisted with that session |
| How is elapsed time reconstructed after process death? | Checked UTC wall interval, because the previous monotonic clock is unavailable |

Machine-local timezone is not an authority in production temporal paths.

## Live reconciliation

A live session begins with both a UTC timestamp and a monotonic anchor. At finish or layer switch:

1. elapsed seconds come from the monotonic anchor;
2. Strata derives the expected UTC endpoint as `started_at_utc + monotonic_elapsed`;
3. that endpoint is compared with the observed UTC wall clock;
4. divergence of five seconds or less is treated as ordinary scheduler/NTP jitter and the monotonic-derived endpoint is committed;
5. larger forward or backward divergence fails visibly before the transition consumes active state.

A failed reconciliation enters the existing persistence-recovery surface. Strata does not clamp a negative interval to zero, cast it to an unsigned duration, or silently choose wall time over monotonic time.

## Restart and unattended recovery

A monotonic anchor cannot be serialized across process death. CLI stop, startup recovery, and checkpoint restoration therefore use a checked UTC wall interval.

- A start later than the observed end is rejected as a future timestamp.
- An unattended interval of seven days or less can be reconstructed normally.
- A longer CLI interval is rejected unless the user deliberately runs `strata stop --accept-clock-jump`.
- The override accepts the recorded wall interval. It does not rewrite timestamps or infer the user's intended correction.
- Reconstructing an `Instant` uses checked subtraction and reports an error if the platform monotonic range cannot represent the interval.

Historical catch-up mutations use their recorded UTC schedule rather than pretending they occurred at current wall time.

## Timezone and civil policy

The current configuration stores a fixed UTC offset in seconds. That offset owns:

- rendered clock labels for newly completed sessions;
- operational-day allocation for new sessions;
- live report previews.

This policy is intentionally deterministic under travel: changing the host machine timezone does not silently change Strata's interpretation. Changing the configured offset changes future civil projection and allocation, but completed sessions retain their persisted operational-day key.

The fixed-offset policy does **not** implement IANA timezone rules or daylight-saving transitions. Tests across winter and summer prove constancy, not DST awareness. Introducing named-zone history would require a separate migration and product decision.

## Reproducible history

Completed sessions store both absolute chronology and an operational-day key. Reports filter that persisted key. Consequently, changing the configured offset or cutoff later does not regroup existing sessions into different historical days.

This does not settle overlap allocation, sunrise claims, or zero-duration transition policy. Those remain TEMPORAL-002.

## Failure matrix

| Condition | Behavior |
|---|---|
| Live wall clock moves backward or forward by more than five seconds | Block transition; preserve active state; show recovery |
| Live wall/monotonic difference is at most five seconds | Commit monotonic duration and monotonic-derived UTC endpoint |
| Persisted start is in the future | Fail without consuming active state |
| Cross-process wall interval exceeds seven days | Require explicit `--accept-clock-jump` for CLI stop |
| Configured UTC offset is invalid | Startup fails before authority resolution |
| Reconstructed monotonic anchor is not representable | Fail visibly rather than panic |
| User travels without changing Strata configuration | Fixed-offset interpretation remains unchanged |
| User changes offset later | New sessions use new policy; historical grouping remains persisted |

## Certification

TEMPORAL-001 adds unit and process coverage for:

- future timestamps;
- backward and forward live clock jumps;
- ordinary wall jitter;
- suspend-like agreement between wall and monotonic elapsed time;
- explicit long-interval acceptance;
- fixed-offset behavior across DST seasons;
- travel/configuration projection changes;
- preservation of active state on rejected legacy stops;
- all existing SQLite authority, lifecycle, recovery, and configuration gates.
