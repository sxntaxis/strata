---
id: SEDIMENT-001D1
kind: work
state: completed
authority: accepted
created: 2026-08-02
updated: 2026-08-02
---

# SEDIMENT-001D1 — explicit snapshot identity and immutable viewing

## Purpose

Stop historical report rendering from treating cumulative persisted state, per-day reconstruction, and mutable preview simulation as interchangeable artifacts.

## Accepted contract

Snapshot identity is explicit in one typed envelope:

- `CumulativeCheckpoint` — authentic canonical sediment as of a capture instant;
- `DailyContribution` — sediment mass attributed to exactly one operational day;
- `DerivedPreview` — deterministic visualization reconstructed from ledger truth for viewing only.

Each envelope records snapshot schema version, semantic kind, optional operational day, source revision, provenance, deterministic idle policy, reconstruction status, and canonical `SandState`.

Historical report rendering is immutable:

- restoration may adapt presentation dimensions only;
- rendering never calls physics update;
- repeated rendering returns identical lines and leaves the artifact unchanged;
- cache identity includes the serialized artifact and viewport rather than grain-vector length alone;
- the UI labels artifact kind, reconstruction status, and idle policy.

Legacy persisted daily payloads are classified explicitly as cumulative legacy checkpoints. They cannot silently substitute for a daily contribution. When no compatible daily contribution exists, the report uses an in-memory `DerivedPreview` reconstructed from canonical session slices. Viewing and rebuilding the preview performs no persistence write or deletion.

## Certified proofs

- all three snapshot kinds are distinct and serializable;
- legacy bare `SandState` is classified as cumulative legacy evidence;
- a daily report rejects cumulative evidence as its daily contribution;
- repeated historical rendering is deterministic;
- rendering leaves coordinates, pending runs, frame count, sweep direction, and RNG unchanged;
- derived source revision changes when chronology material changes;
- idle inclusion is explicit and deterministic;
- missing or incompatible daily artifacts fall back to a marked derived preview without persistence mutation;
- formatting, strict Clippy, and the full all-features suite pass;
- all prior persistence, recovery, report, CLI, and TUI gates remain green.

## Durable authority

- `docs/SEDIMENT_AUTHORITY.md` records snapshot kinds and immutable viewing;
- `docs/ARCHITECTURE.md` assigns identity/provenance to `src/sand/snapshot.rs`;
- STRATA-D030 and STRATA-D031 constrain snapshot substitution and viewing;
- report UI exposes the artifact's semantic status.

## Boundary

This unit does not change the database/file storage key for daily artifacts and does not close issue #18. SEDIMENT-001D2 must introduce authoritative daily-contribution persistence, source-revision comparison, correct edit/delete invalidation, and explicit disposition of legacy cumulative daily rows.
