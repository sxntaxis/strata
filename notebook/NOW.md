---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-02
authority: working
summary: Persistence, configuration, clock authority, and interval-boundary semantics are complete; domain reconciliation now leads the frontier.
next: Implement DOMAIN-001 for issues #2 and #12 residuals: project/classification authority and explicit idle semantics.
---

# NOW — Strata

## Current phase

The SQLite migration program, AUTHORITY-001, TEMPORAL-001, and TEMPORAL-002 are complete. Strata now has durable authority, fail-closed configuration, explicit clock roles, reproducible fixed-clock boundaries, exact overlap allocation, and truthful zero-transition semantics.

The project is moving from **temporal foundations** to **domain and projection correctness**.

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

- SQLite schema version 5 is authoritative after explicit activation.
- CLI and TUI share repository, runtime-coordination, configuration, and temporal boundaries.
- Live duration is monotonic; persisted absolute timestamps are UTC.
- Clock discontinuities fail closed; future starts and unsafe unattended intervals require explicit handling.
- The only supported operational-day policy is a fixed clock under a fixed UTC offset.
- New sessions capture offset and boundary-minute provenance.
- Canonical sessions remain single identities while reports allocate exact overlap slices across operational days.
- Exact-boundary endpoints create no empty fragments and allocated seconds are conserved.
- Existing `sunrise` configuration is migrated visibly to fixed policy; no solar behavior is claimed.
- Zero-whole-second finishes and switches retain transactional receipts but create no work rows.
- Old legacy rows remain readable without invented chronology; new CSV and bundle formats preserve temporal provenance.
- Persistence failure freezes mutation and offers retry, reload, emergency export, safe exit, or explicit unsafe exit.

## Completed post-migration units

- **AUTHORITY-001** — issue #21: shared validated settings and fail-closed CLI/TUI startup configuration.
- **TEMPORAL-001** — issue #25: clock roles, discontinuity handling, fixed-offset civil authority, and reproducible history.
- **TEMPORAL-002** — issues #4, #23, #27: overlap allocation, removal of false sunrise semantics, and zero-transition policy.

Complete profile isolation remains open under issue #15. IANA timezone/DST adoption remains a separate future decision.

## Active sequence

1. **DOMAIN-001** — issues #2 and #12 residuals: project/classification model and explicit idle semantics.
2. **REPORT-001** — issues #1, #14, #17, #28: custom ranges, provisional active time, valid ICS, deterministic ordering.
3. **SEDIMENT-001** — issues #6, #7, #16, #18, #26: conserved logical sediment independent of viewport and mutable previews.
4. **INTERACTION-001** — issues #19, #20, #24: explicit edit modes, terminal lifecycle guard, truthful keybinding policy.

## Current risks

- Project strings are preserved but do not yet have one complete product/domain authority.
- The accepted idle rename is not yet reflected consistently in runtime vocabulary and classification rules.
- Reports and exports still need custom ranges, provisional active-time policy, valid ICS, and deterministic ordering.
- Sediment rendering, resize, catch-up, and snapshots still lack one conservation model.

## Next

Implement **DOMAIN-001**. Reconcile issues #2 and #12 against current SQLite and temporal behavior before changing schema or terminology; preserve canonical interval identity and overlap semantics.
