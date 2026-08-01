---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-01
authority: working
summary: SQLite authority and fail-closed startup configuration are complete; Strata is entering temporal, reporting, and sediment correctness work.
next: Implement TEMPORAL-001 for issue #25 so elapsed duration, wall-clock timestamps, timezone, clock jumps, and historical operational-day interpretation obey one explicit contract.
---

# NOW — Strata

## Current phase

The SQLite migration program is complete at 9/9 acceptance criteria. AUTHORITY-001 now prevents invalid configuration from silently redirecting either interface to default storage or time settings.

The project is moving from **persistence and startup authority** to **temporal correctness on top of that authority**.

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
- AUTHORITY-001 loads one validated startup configuration before choosing CLI/TUI or resolving data authority.
- Invalid JSON, key/action data, operational settings, UTC offsets, and configured legacy paths fail visibly before writable authority is opened.
- `--ignore-config` is the explicit deliberate-default bypass; TUI hot reload retains the last valid settings on failure.

## Completed post-migration unit

- **AUTHORITY-001** — issue #21: shared validated settings and fail-closed CLI/TUI startup configuration.

Complete profile isolation and runtime switching remain open under issue #15; they were not hidden inside the startup-validation unit.

## Active sequence

1. **TEMPORAL-001** — issue #25: monotonic/wall-clock reconciliation, timezone authority, future/negative interval handling, reproducible historical boundaries.
2. **TEMPORAL-002** — issues #4, #23, #27: overlap allocation, honest sunrise policy, zero-duration transitions.
3. **DOMAIN-001** — issues #2 and #12 residuals: project/classification model and explicit idle semantics.
4. **REPORT-001** — issues #1, #14, #17, #28: custom ranges, provisional active time, valid ICS, deterministic ordering.
5. **SEDIMENT-001** — issues #6, #7, #16, #18, #26: conserved logical sediment independent of viewport and mutable previews.
6. **INTERACTION-001** — issues #19, #20, #24: explicit edit modes, terminal lifecycle guard, truthful keybinding policy.

## Issue reconciliation state

The pre-SQLite issue queue must be read against current code, not its original premises. Some issues are fully or partly satisfied by the SQLite and authority programs; none should be closed solely because their storage premise changed. `work/ISSUE-RECONCILIATION-001.md` records the current disposition.

## Current risks

- Timekeeping lacks one explicit authority under clock jumps, suspend, and timezone changes.
- Historical operational-day reports may not remain reproducible after settings changes.
- Reports and exports may remain semantically inconsistent despite durable storage.
- Sediment rendering, resize, catch-up, and snapshots still lack one conservation model.
- The accepted idle rename is not yet reflected consistently in runtime vocabulary.

## Next

Implement **TEMPORAL-001** on issue #25. Negative or future wall-clock intervals must never become ordinary work, invalid temporal settings must remain fail-closed, and live/committed duration must follow one documented reconciliation policy.
