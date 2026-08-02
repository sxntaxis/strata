---
id: SEDIMENT-001D2
kind: work
state: active
authority: working
created: 2026-08-02
updated: 2026-08-02
---

# SEDIMENT-001D2 — daily contribution persistence and invalidation

## Objective

Persist historical daily sediment as truthful typed contributions derived from canonical ledger slices, not as cumulative live-state copies.

## Required contract

- Persist `DailyContribution` envelopes under daily snapshot keys.
- Derive payload and source revision from exact operational-day session slices.
- Idle inclusion is explicit and deterministic.
- Trust persisted artifacts only when schema, kind, day, and source revision match current ledger truth.
- Rebuild stale or missing contributions deterministically.
- Session deletion invalidates every operational day touched by the canonical session.
- Description-only edits do not invalidate sediment mass.
- Chronology/category/duration changes, if introduced later, must invalidate all before/after affected days.
- Legacy cumulative daily rows are evidence: archive or replace them explicitly, never reinterpret them as daily contributions.
- SQLite and legacy-file authority follow equivalent read, validate, rebuild, publish, and legacy-disposition rules.
- Derived previews remain in-memory fallback and never become authority without an explicit typed persistence step.

## Acceptance proofs

- current-day cumulative state is never written as a daily contribution;
- cross-boundary sessions contribute exact conserved seconds to each day;
- persisted contribution round-trips with kind, day, revision, provenance, idle policy, and state;
- matching revision reuses the artifact;
- mismatched revision rebuilds and replaces it;
- deleting a cross-boundary session rebuilds every affected day;
- description-only edit leaves source revision unchanged;
- legacy bare daily rows are explicitly displaced without silent reinterpretation;
- SQLite and file paths produce equivalent envelopes;
- issue #18 closes only after full certification.
