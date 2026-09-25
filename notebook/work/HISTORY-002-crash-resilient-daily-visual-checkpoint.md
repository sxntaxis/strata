---
id: HISTORY-002
kind: work
state: completed
authority: accepted
created: 2026-09-25
updated: 2026-09-25
summary: Preserve the latest canonical sand topology as a historical day artifact when the machine is off at the operational-day cutoff.
---

# HISTORY-002 — crash-resilient daily visual checkpoint

## Owner decision

If Strata is not running at the operational-day boundary, the day's historical visual should use the latest canonical sand photo already saved for that day. It must preserve the actual saved topology and identify that it is the latest available checkpoint rather than claiming it was captured at the boundary.

## Authority separation

- `sand_state` remains the current live canonical formation.
- `sand_snapshots` `daily` stores a canonical daily visual checkpoint, refreshed by autosave until the observed cutoff freezes the final day-end image.
- `daily-contribution` remains ledger-derived accounting evidence.
- `DerivedPreview` is a read-only visual fallback only if that day has no saved canonical image.

## Implementation and proof obligations

- Autosave publishes the current canonical `SandState` as that operational day's latest visual checkpoint.
- A later autosave may replace an earlier latest checkpoint, but may not overwrite an already finalized day-end or legacy canonical artifact.
- Crossing the cutoff promotes the exact boundary state over the mutable latest checkpoint and makes it immutable.
- Historical selection prefers either canonical capture over a ledger reconstruction and labels latest-available captures truthfully.
- SQLite writes are transactional; failures remain visible through existing persistence-recovery controls.
- Verify checkpoint replacement, finalization, immutable historical rendering, ledger-preview fallback, formatting, strict Clippy, and the full test suite.

## Result

Implementation is certified on latest GitHub `main`. Formatting, strict Clippy, 515 unit tests, 24 integration tests, CLI help smoke, latest-photo replacement, final day-end promotion/immutability, SQLite transaction rollback, and diff hygiene pass. No SQLite schema migration is required. The original checkout's pre-existing working changes remain untouched; the owner's profile database was inspected read-only only.
