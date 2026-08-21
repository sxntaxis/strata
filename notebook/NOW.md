---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-09
authority: working
summary: ARCH-001 is complete: SQLite is the sole runtime persistence authority and the transitional file authority is retired.
next: Maintain the certified SQLite-only baseline; begin only a newly justified product issue or bounded work unit.
---

# NOW — Strata

## Current phase

ARCH-001 has been completed. The post-SQLite issue reconciliation program is complete, and the current runtime is SQLite-only.

The certified system includes:

- fail-closed profile-bound SQLite runtime persistence with one current schema;
- monotonic/UTC/fixed-offset time and exact operational-day allocation;
- canonical category/layer, session, active-generation, and report identity;
- conserved sediment, bounded recovery, immutable historical artifacts, and revision-matched daily contributions;
- receipt-governed switch/finish/reset transitions plus atomic receipt-free clear-all;
- active/archived category integrity with stable archive/restore identity;
- explicit report editing, truthful keymap/palette/atlas routing, and exactly-once terminal restoration;
- session-owned active description drafts separated from durable category metadata and reusable tags;
- one process-bound profile UUID owning complete data, state, configuration, recovery, and SQLite paths;
- real process proofs for profile isolation, copied-artifact refusal, persistence failure, live control, and PTY restoration.

The transitional CSV/JSON runtime, authority selection, activation ceremony, and historical schema
upgrade chain are retired. Portable bundle export/import and SQLite doctor, backup, and restore remain
product functionality. Runtime recovery, checkpoints, receipts, categories, sessions, and sediment are
SQLite-owned.

## Completed post-migration units

- **AUTHORITY-001** — issue #21.
- **AUTHORITY-002** — issues #22 and #15.
- **TEMPORAL-001** — issue #25.
- **TEMPORAL-002** — issues #4, #23, #27.
- **DOMAIN-001** — issues #2, #12.
- **REPORT-001** — issues #1, #3, #14, #17, #28.
- **SEDIMENT-001** — issues #6, #7, #16, #18, #26.
- **INTERACTION-001A** — issue #19.
- **INTERACTION-001B** — issue #20.
- **INTERACTION-001C** — issue #24.
- **RECONCILIATION-001A** — issue #5 and historical-meaning portion of #13.
- **RECONCILIATION-001B1/B2A/B2B/B2C/B3A/B3B/B3C** — issue #10.
- **RECONCILIATION-001C1/C2** — issue #13.

## Verified final baseline

- formatting and strict Clippy pass;
- `cargo test --all-features` passes with 194 library tests plus the integration, process, and PTY suites;
- fresh-profile direct-SQLite smoke proof passes;
- CLI help exposes no retired migration or activation commands;
- GitHub Actions run 31340435742 passes on ARCH-001 head `6f7f7df7808e4e919a92a206eb71559b896378a0`.

## Certification evidence

- current schema initializes fresh databases transactionally at `user_version = 1` and rejects other development versions;
- strict storage-authority residue search is empty outside the authoritative decision record;
- formatting, strict Clippy, tests, fresh-profile smoke proof, help output, and diff hygiene were run for ARCH-001.

## Known non-blocking questions

The accepted implementation does not settle every possible future product direction. Remaining design questions include vertical chronology, optional category relationships, final Karma terminology, future sediment clearing/formation semantics, zoom/compression/panning, configurable quantum migration, possible IANA timezone support, and any future stable identity for queued cross-authority mutation replay.

These are not open implementation defects. They require new evidence and an explicit future unit before constraining the current system.

## Next

Preserve the certified baseline. New work begins only from a newly justified issue, decision, or architecture unit; superseded issue premises remain in Git history rather than current authority.
