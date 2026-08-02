---
id: SEDIMENT-001C2
kind: work
state: active
authority: working
created: 2026-08-02
updated: 2026-08-02
---

# SEDIMENT-001C2 — durable bounded detached recovery

## Issue

Issue #6: detached recovery currently replays missed spawn and physics events, relaxes a replacement topology, and does not provide one equivalent retry-safe evidence lifecycle across SQLite and legacy-file authority.

## Selected contract

- A checkpoint is claimed and validated before recovery changes authoritative state.
- The checkpoint's canonical sediment topology is restored directly.
- Recovery uses a fixed target UTC recorded before publication so retries derive the same result from the same checkpoint evidence.
- Elapsed contribution is segmented only by persisted mutation events; work is proportional to mutation count and current ingress width, never detached duration.
- Each segment calculates due spawn count and accumulator remainder through checked periodic arithmetic.
- Missed grains are added through compressed category/count runs.
- Missed physics frames are not replayed and do not relax restored topology. Their accumulator remainder advances exactly; ordinary live physics resumes afterward.
- Persisted category switches and clearing mutations are applied once in recorded chronological order.
- SQLite publishes recovered sediment, daily snapshot, active-session continuity, and checkpoint commit through its existing atomic recovery transaction, then clears committed evidence.
- Legacy-file recovery keeps the checkpoint until deterministic recovered state and snapshot are durably published. A fixed recovery target makes retry after partial publication idempotent.
- Invalid schema, future timestamps, stale active identity, impossible accumulator state, overflow, or malformed mutation order fail closed without deleting evidence.
- Successful startup has no synthetic replay backlog and no relaxed catch-up replacement topology.

## Acceptance proofs

- short gaps produce exact mass and accumulator remainder;
- extreme gaps produce bounded run count and bounded execution work;
- pre-detach grain coordinates and topology remain exact;
- category changes split recovered mass at the correct timestamps;
- clear mutations operate once and in order;
- repeated reopen after success adds no duplicate mass;
- interruption before commit retains reclaimable evidence;
- interruption after deterministic legacy publication but before checkpoint deletion retries to the same state;
- stale, future, malformed, and mismatched checkpoints fail closed;
- SQLite recovery remains atomic and reclaimable;
- legacy and SQLite paths follow equivalent claim → derive → publish → clear semantics;
- all prior persistence, temporal, report, sediment, CLI, and TUI gates remain green.

## Boundaries

- No attempt is made to reconstruct every missed visual physics frame.
- No global relaxation or catch-up topology replacement is permitted.
- No historical snapshot-kind redesign; that remains SEDIMENT-001D / issue #18.
- No new user-facing recovery editor or conflict UI.
