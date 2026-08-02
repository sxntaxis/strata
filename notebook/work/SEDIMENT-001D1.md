---
id: SEDIMENT-001D1
kind: work
state: active
authority: working
created: 2026-08-02
updated: 2026-08-02
---

# SEDIMENT-001D1 — explicit snapshot identity and immutable viewing

## Purpose

Stop historical report rendering from treating cumulative persisted state, per-day reconstruction, and mutable preview simulation as interchangeable artifacts.

## Selected contract

Snapshot identity is explicit in one typed envelope:

- `CumulativeCheckpoint` — authentic canonical sediment as of a capture instant;
- `DailyContribution` — sediment mass attributed to exactly one operational day;
- `DerivedPreview` — deterministic visualization reconstructed from ledger truth for viewing only.

Each envelope records:

- snapshot schema version;
- semantic kind;
- optional operational day;
- source revision;
- provenance;
- deterministic idle-inclusion policy;
- whether the artifact is reconstructed;
- canonical `SandState` payload.

Historical report rendering is immutable:

- restoring an artifact into a viewport engine may adapt presentation dimensions only;
- rendering never calls physics update;
- repeated rendering returns identical lines and leaves the artifact unchanged;
- preview cache identity includes kind, day, revision, and full logical mass rather than grain-vector length alone.

Legacy persisted daily payloads are classified explicitly as cumulative legacy checkpoints. They cannot silently substitute for a daily contribution. When no valid daily contribution exists, the report uses an in-memory `DerivedPreview` reconstructed from session slices. D1 does not overwrite or delete the legacy payload.

## Acceptance proofs

- all three snapshot kinds are distinct and serializable;
- legacy bare `SandState` is classified as cumulative legacy evidence;
- a daily report rejects a cumulative artifact as its daily contribution;
- repeated historical rendering is byte-for-byte stable;
- rendering does not change coordinates, pending runs, frame count, sweep direction, or RNG;
- derived preview source revision changes when day-owned session chronology changes;
- idle inclusion is explicit and deterministic;
- missing or incompatible persisted snapshot falls back to a marked derived preview without persistence mutation;
- all prior sediment, persistence, recovery, report, CLI, and TUI gates remain green.

## Boundary

This unit does not change the database/file storage key for daily artifacts and does not close issue #18. SEDIMENT-001D2 will introduce authoritative daily-contribution persistence, correct edit/delete invalidation, and explicit disposition of legacy cumulative daily rows.
