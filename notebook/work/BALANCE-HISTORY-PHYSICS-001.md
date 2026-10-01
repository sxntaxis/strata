---
id: BALANCE-HISTORY-PHYSICS-001
kind: work
state: candidate
authority: working
created: 2026-09-27
updated: 2026-09-27
---

# BALANCE-HISTORY-PHYSICS-001 — disposable historical snapshot animation

## Owner-approved contract

Historical Balance may animate a saved sediment artifact purely as visual view state. The saved artifact is never rewritten.

```text
authoritative historical snapshot
        -> fresh temporary production-Classic preview
        -> gravity/settling only
        -> discard on navigation or Balance exit
```

The preview never becomes authority. It never replaces the live sand engine, creates rain/ingress, publishes checkpoint state, writes SQLite, or persists its evolved topology.

Leaving a historical interval destroys its preview. Returning to the same interval creates a new preview from the original saved snapshot, so there is deliberately no continuity between visits. Closing and reopening Balance has the same reset semantics.

## Physics boundary

The preview uses the production Classic restore path and production gravity logic. A dedicated physics-only step advances the normal Classic frame/gravity cadence without flushing pending drive. A dedicated preview resize path preserves the same resize mechanics without materializing pending logical sediment. Ordinary production `update()` and `resize()` keep their existing ingress behavior.

If Classic preview construction cannot restore a historical artifact, Balance falls back to the existing immutable snapshot renderer rather than blocking the report surface.

## Authority boundary

Snapshot kind, provenance, reconstruction status, source revision, idle policy, persisted dimensions, and source topology remain unchanged. Only the temporary clone evolves while visible.
