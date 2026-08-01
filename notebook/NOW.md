---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-01
authority: working
summary: SQLite authority migration is complete; Strata is entering post-migration authority, temporal, reporting, and sediment correctness work.
next: Implement AUTHORITY-001 for issue #21 so invalid configuration cannot redirect CLI or TUI operations to a default database or time policy.
---

# NOW — Strata

## Current phase

The SQLite migration program is complete at 9/9 acceptance criteria. After explicit activation, CLI and TUI share one transactional SQLite authority with deterministic interchange, maintenance, recovery, and legacy-evidence custody.

The project is moving from **persistence architecture** to **product correctness on top of that authority**.

## Accepted product baseline

- Strata is a continuous temporal ledger and an active timer.
- Time always passes; the accepted baseline name is **idle**.
- Idle continues depositing sediment but is omitted from ordinary active-time accounting.
- Strata is general-purpose rather than freelancing-specific.
- Exact chronological history and accountable sedimentary history are both meaningful.
- Sediment is product function and artwork, not disposable decoration.
- One grain currently represents one elapsed second.
- Braille-cell color mixing is intentional.

## Verified technical baseline

- Issue #8 is closed at 9/9 PASS.
- SQLite schema version 4 is the current authoritative model after activation.
- CLI and TUI share repository and runtime-coordination boundaries.
- Start, finish, switch, reset, detach, and recovery are fenced, transactional, and retry-safe.
- Persistence failure freezes mutation and provides retry, reload, emergency export, safe exit, or explicit unsafe exit.
- Deterministic CSV bundle export/import, dry-run validation, doctor, backup, restore, and legacy custody are implemented.
- Legacy CSV/JSON is pre-activation authority or post-activation evidence; it is never dual-written after activation.

## Active sequence

1. **AUTHORITY-001** — issue #21: shared validated settings, fail-closed CLI/TUI configuration, explicit bypass/profile selection.
2. **TEMPORAL-001** — issue #25: monotonic/wall-clock reconciliation, timezone authority, future/negative interval handling, reproducible historical boundaries.
3. **TEMPORAL-002** — issues #4, #23, #27: overlap allocation, honest sunrise policy, zero-duration transitions.
4. **DOMAIN-001** — issues #2 and #12 residuals: project/classification model and explicit idle semantics.
5. **REPORT-001** — issues #1, #14, #17, #28: custom ranges, provisional active time, valid ICS, deterministic ordering.
6. **SEDIMENT-001** — issues #6, #7, #16, #18, #26: conserved logical sediment independent of viewport and mutable previews.
7. **INTERACTION-001** — issues #19, #20, #24: explicit edit modes, terminal lifecycle guard, truthful keybinding policy.

## Issue reconciliation state

The pre-SQLite issue queue must be read against current code, not its original premises. Some issues are fully or partly satisfied by SQLITE-001 through SQLITE-012; none should be closed solely because their storage premise changed. `work/ISSUE-RECONCILIATION-001.md` records the current disposition.

## Current risks

- Invalid configuration may silently select a default database/time policy.
- Timekeeping still lacks one explicit authority under clock and timezone changes.
- Reports and exports may remain semantically inconsistent despite durable storage.
- Sediment rendering, resize, catch-up, and snapshots still lack one conservation model.
- The accepted idle rename is not yet reflected consistently in runtime vocabulary.

## Next

Implement **AUTHORITY-001** on issue #21. No command may open or mutate a database until authority-critical configuration is validated or an explicit bypass/profile override is selected.
