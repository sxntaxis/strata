---
id: SEDIMENT-001C2
kind: work
state: completed
authority: accepted
created: 2026-08-02
updated: 2026-08-02
---

# SEDIMENT-001C2 — durable bounded detached recovery

## Issue

Issue #6: detached recovery replayed missed spawn and physics events, relaxed a replacement topology, and lacked one equivalent retry-safe evidence lifecycle across SQLite and legacy-file authority.

## Accepted contract

- Runtime checkpoints cover periodic autosave, detach, terminal closure, and crash recovery.
- A checkpoint is claimed and validated before recovered state is published.
- The checkpoint's canonical sediment topology and engine metadata restore directly.
- A recovery target UTC is persisted before the first recovery publication.
- Elapsed contribution is calculated once with checked periodic arithmetic.
- Missed grains are appended as compressed category/count runs.
- Missed physics frames are counted but never replayed; only the accumulator remainder advances.
- Recovery work is independent of detached duration apart from validation and compact run changes.
- SQLite publishes recovered sediment, daily snapshot, active-session continuity, and checkpoint status atomically.
- Committed SQLite evidence remains reclaimable until successful startup replaces it with a fresh pending checkpoint.
- Legacy-file recovery persists the target before derivation and a committed marker after deterministic state publication.
- Reopening committed evidence re-derives from the preserved base to the new startup time and overwrites authoritative recovered state rather than adding to it.
- Normal shutdown may retire pending or committed evidence, but recovering and quarantined evidence remain protected.
- Invalid schemas, timestamps, identities, coordinates, accumulators, and arithmetic fail closed without deleting unresolved evidence.
- Successful startup has no synthetic replay backlog and no relaxed catch-up replacement topology.

## Queued-mutation boundary

This unit does not claim stable queued-mutation replay.

- New runtime checkpoints are refused while mutations are pending.
- Legacy checkpoints that already contain queued mutations fail closed and retain evidence.
- A future implementation would require one stable receipt identity and equivalent idempotent semantics across SQLite and legacy-file authority.

## Certified proofs

- short gaps produce exact mass and accumulator remainder;
- a billion-second gap produces one compressed pending run;
- pre-checkpoint grain coordinates, frame count, sweep direction, and RNG state remain exact;
- malformed sediment and invalid periodic inputs fail closed;
- checkpoint schema fields remain backward-compatible;
- committed SQLite evidence is reclaimable and published atomically;
- pending checkpoints can be retired on normal shutdown;
- recovering evidence cannot be cleared by normal shutdown;
- normal activated TUI exit succeeds;
- post-commit reload retry preserves committed history and exits;
- formatting and strict Clippy pass with all targets and features;
- 153 unit tests, 9 CLI lifecycle tests, 6 configuration tests, 1 report-help test, 12 SQLite/TUI process tests, 2 temporal tests, and doc tests pass.

## Durable authority

- `docs/SEDIMENT_AUTHORITY.md` records runtime checkpoint custody and bounded recovery semantics;
- `docs/ARCHITECTURE.md` assigns recovery custody to application orchestration plus SQLite/file persistence;
- STRATA-D028 and STRATA-D029 constrain recovery and evidence retirement;
- `notebook/work/SEDIMENT-001.md` advances to snapshot identity.

## Boundaries

- No missed visual physics frames are reconstructed.
- No global relaxation or catch-up topology replacement is permitted.
- No queued-mutation replay without stable receipts.
- No historical snapshot-kind redesign; that remains SEDIMENT-001D / issue #18.
- No new user-facing recovery editor or conflict UI.
