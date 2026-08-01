---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-01
authority: working
summary: SQLite, fail-closed configuration, and explicit temporal authority are complete; remaining interval semantics now lead the frontier.
next: Implement TEMPORAL-002 for issues #4, #23, and #27: overlap allocation, honest sunrise behavior, and zero-duration transitions.
---

# NOW — Strata

## Current phase

The SQLite migration program, AUTHORITY-001, and TEMPORAL-001 are complete. Strata now has durable persistence, fail-closed startup selection, and one documented contract for monotonic duration, UTC timestamps, fixed-offset civil time, clock discontinuities, and historical operational-day grouping.

The project is moving from **authority foundations** to **remaining interval semantics and product correctness**.

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

- SQLite schema version 4 is authoritative after explicit activation.
- CLI and TUI share repository, runtime-coordination, configuration, and temporal boundaries.
- Invalid configuration fails before writable authority; `--ignore-config` is explicit.
- Live duration uses monotonic elapsed time; persisted absolute timestamps use UTC.
- Live wall/monotonic divergence above five seconds blocks the transition and preserves active state.
- Future persisted starts are rejected rather than clamped or cast.
- Cross-process intervals above seven days require explicit `stop --accept-clock-jump` confirmation.
- New civil labels and operational days use the validated fixed UTC offset, not the host machine timezone.
- Historical reports use persisted operational-day keys and remain grouped after later offset changes.
- The current fixed-offset policy is deterministic but does not claim IANA/DST behavior.
- Persistence failure freezes mutation and offers retry, reload, emergency export, safe exit, or explicit unsafe exit.

## Completed post-migration units

- **AUTHORITY-001** — issue #21: shared validated settings and fail-closed CLI/TUI startup configuration.
- **TEMPORAL-001** — issue #25: explicit clock roles, discontinuity handling, fixed-offset civil authority, and reproducible historical grouping.

Complete profile isolation remains open under issue #15. IANA timezone/DST adoption is not implied by TEMPORAL-001.

## Active sequence

1. **TEMPORAL-002** — issues #4, #23, #27: overlap allocation, honest sunrise policy, zero-duration transitions.
2. **DOMAIN-001** — issues #2 and #12 residuals: project/classification model and explicit idle semantics.
3. **REPORT-001** — issues #1, #14, #17, #28: custom ranges, provisional active time, valid ICS, deterministic ordering.
4. **SEDIMENT-001** — issues #6, #7, #16, #18, #26: conserved logical sediment independent of viewport and mutable previews.
5. **INTERACTION-001** — issues #19, #20, #24: explicit edit modes, terminal lifecycle guard, truthful keybinding policy.

## Current risks

- Intervals spanning operational-day boundaries still need one allocation rule.
- `sunrise` currently names fixed-cutoff behavior and must become honest.
- Zero-duration switches need an explicit storage and sediment policy.
- Reports and exports may remain semantically inconsistent despite durable storage.
- Sediment rendering, resize, catch-up, and snapshots still lack one conservation model.
- The accepted idle rename is not yet reflected consistently in runtime vocabulary.

## Next

Implement **TEMPORAL-002**. Do not broaden it into report UI, project taxonomy, or sediment topology; establish the remaining interval rules first so later projections inherit one truthful chronology.
