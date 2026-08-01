from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if old not in text:
        raise SystemExit(f"missing documentation anchor in {path}: {old[:180]!r}")
    target.write_text(text.replace(old, new, 1))


readme_section = r'''
## Time authority

Strata uses distinct clocks for distinct truths:

- **Live elapsed duration** uses the process monotonic clock.
- **Persisted timestamps** use UTC.
- **Civil start/end rendering and operational-day allocation** use the validated fixed UTC offset from `keymap.json`.
- **Historical report grouping** uses the operational-day key persisted with each completed session; later offset changes do not regroup old history.

At a live finish or layer switch, Strata reconciles monotonic elapsed time against observed UTC wall time. A divergence greater than five seconds is treated as a clock discontinuity: the transition fails visibly and active state remains available for recovery rather than being converted into ordinary work.

CLI stops and recovered sessions cannot reconstruct a cross-process monotonic clock, so they use a checked UTC wall interval. Future starts are rejected. An unattended interval above seven days requires explicit confirmation:

```bash
strata stop --accept-clock-jump
```

Use that override only after inspecting the active timestamp and system clock; it accepts the recorded wall interval rather than guessing a correction.

The current policy is a **fixed offset**, not an IANA timezone. It is deterministic across travel and seasonal clock changes but does not automatically apply daylight-saving transitions. Sunrise semantics remain separate work. The full contract is recorded in [`docs/TEMPORAL_AUTHORITY.md`](docs/TEMPORAL_AUTHORITY.md).

'''
replace_once(
    "README.md",
    "## Persistence authority\n",
    readme_section + "## Persistence authority\n",
)

replace_once(
    "docs/ARCHITECTURE.md",
    "- `src/domain.rs` — categories, sessions, operational-day logic, and report aggregation.\n",
    "- `src/domain.rs` — categories, sessions, operational-day logic, and report aggregation.\n- `src/temporal.rs` — checked wall intervals, monotonic/wall reconciliation, fixed-offset civil projection, and operational-day allocation.\n",
)

architecture_temporal = r'''
TEMPORAL-001 establishes one explicit temporal authority:

- live elapsed duration is owned by the process monotonic clock;
- UTC owns persisted absolute timestamps;
- a live transition compares observed UTC with the UTC endpoint implied by monotonic elapsed time;
- divergence above five seconds fails closed and preserves active state;
- cross-process recovery uses checked UTC wall intervals because monotonic state cannot survive process death;
- future starts are rejected, and unattended intervals above seven days require explicit CLI confirmation;
- the validated fixed UTC offset owns civil display and new operational-day allocation;
- the operational-day key persisted with a session owns historical report grouping after later setting changes;
- the fixed-offset policy is deliberately not an IANA/DST policy.

The detailed contract and failure matrix are `docs/TEMPORAL_AUTHORITY.md`.

'''
replace_once(
    "docs/ARCHITECTURE.md",
    "The SQLite closure evidence is `docs/SQLITE_MIGRATION_CLOSURE_AUDIT.md`.\n",
    architecture_temporal
    + "The SQLite closure evidence is `docs/SQLITE_MIGRATION_CLOSURE_AUDIT.md`.\n",
)
replace_once(
    "docs/ARCHITECTURE.md",
    "Persistence structure and startup configuration fallback are no longer the primary risks. The next program is temporal correctness:\n\n1. establish one explicit time authority and wall-clock-jump policy;\n2. define timezone and historical operational-day reproducibility;\n3. correct interval allocation, reporting, export, and classification semantics;\n4. establish a conserved sediment model independent of viewport and mutable previews.\n",
    "Persistence structure, startup configuration fallback, and clock authority are no longer the primary risks. The next program begins with remaining interval semantics:\n\n1. define overlap allocation, honest sunrise behavior, and zero-duration transitions;\n2. correct reporting, export, and classification semantics;\n3. establish a conserved sediment model independent of viewport and mutable previews.\n",
)

replace_once(
    "docs/DECISIONS.md",
    "| STRATA-D013 | CLI and TUI share one validated startup configuration; invalid configuration blocks authority resolution unless `--ignore-config` is explicitly supplied. | implemented and certified |\n",
    "| STRATA-D013 | CLI and TUI share one validated startup configuration; invalid configuration blocks authority resolution unless `--ignore-config` is explicitly supplied. | implemented and certified |\n| STRATA-D014 | Live duration is monotonic; persisted timestamps are UTC; civil projection uses the validated fixed offset; persisted operational-day keys own historical grouping; ambiguous clock discontinuities fail closed. | implemented and certified |\n",
)
replace_once(
    "docs/DECISIONS.md",
    "- timezone and historical operational-day policy to be established by TEMPORAL-001.\n",
    "- future adoption of IANA timezone/DST semantics, if any; the implemented authority is fixed-offset.\n",
)

Path("docs/TEMPORAL_AUTHORITY.md").write_text(r'''# Temporal authority

Status: accepted and certified
Implemented by: TEMPORAL-001
Issue: #25
Last reviewed: 2026-08-01

## Purpose

Strata must preserve interval meaning when wall clocks jump, processes restart, users travel, or configuration changes. No single clock can answer every temporal question, so authority is divided explicitly by responsibility.

## Clock roles

| Question | Authority |
|---|---|
| How much time elapsed while the TUI process remained live? | `std::time::Instant` monotonic elapsed time |
| What absolute timestamps are persisted? | UTC |
| How are new start/end clock labels rendered? | Validated configured fixed UTC offset |
| Which operational day receives a newly completed session? | UTC endpoint projected through that fixed offset and configured cutoff |
| Which day contains an existing historical session? | The operational-day key persisted with that session |
| How is elapsed time reconstructed after process death? | Checked UTC wall interval, because the previous monotonic clock is unavailable |

Machine-local timezone is not an authority in production temporal paths.

## Live reconciliation

A live session begins with both a UTC timestamp and a monotonic anchor. At finish or layer switch:

1. elapsed seconds come from the monotonic anchor;
2. Strata derives the expected UTC endpoint as `started_at_utc + monotonic_elapsed`;
3. that endpoint is compared with the observed UTC wall clock;
4. divergence of five seconds or less is treated as ordinary scheduler/NTP jitter and the monotonic-derived endpoint is committed;
5. larger forward or backward divergence fails visibly before the transition consumes active state.

A failed reconciliation enters the existing persistence-recovery surface. Strata does not clamp a negative interval to zero, cast it to an unsigned duration, or silently choose wall time over monotonic time.

## Restart and unattended recovery

A monotonic anchor cannot be serialized across process death. CLI stop, startup recovery, and checkpoint restoration therefore use a checked UTC wall interval.

- A start later than the observed end is rejected as a future timestamp.
- An unattended interval of seven days or less can be reconstructed normally.
- A longer CLI interval is rejected unless the user deliberately runs `strata stop --accept-clock-jump`.
- The override accepts the recorded wall interval. It does not rewrite timestamps or infer the user's intended correction.
- Reconstructing an `Instant` uses checked subtraction and reports an error if the platform monotonic range cannot represent the interval.

Historical catch-up mutations use their recorded UTC schedule rather than pretending they occurred at current wall time.

## Timezone and civil policy

The current configuration stores a fixed UTC offset in seconds. That offset owns:

- rendered clock labels for newly completed sessions;
- operational-day allocation for new sessions;
- live report previews.

This policy is intentionally deterministic under travel: changing the host machine timezone does not silently change Strata's interpretation. Changing the configured offset changes future civil projection and allocation, but completed sessions retain their persisted operational-day key.

The fixed-offset policy does **not** implement IANA timezone rules or daylight-saving transitions. Tests across winter and summer prove constancy, not DST awareness. Introducing named-zone history would require a separate migration and product decision.

## Reproducible history

Completed sessions store both absolute chronology and an operational-day key. Reports filter that persisted key. Consequently, changing the configured offset or cutoff later does not regroup existing sessions into different historical days.

This does not settle overlap allocation, sunrise claims, or zero-duration transition policy. Those remain TEMPORAL-002.

## Failure matrix

| Condition | Behavior |
|---|---|
| Live wall clock moves backward or forward by more than five seconds | Block transition; preserve active state; show recovery |
| Live wall/monotonic difference is at most five seconds | Commit monotonic duration and monotonic-derived UTC endpoint |
| Persisted start is in the future | Fail without consuming active state |
| Cross-process wall interval exceeds seven days | Require explicit `--accept-clock-jump` for CLI stop |
| Configured UTC offset is invalid | Startup fails before authority resolution |
| Reconstructed monotonic anchor is not representable | Fail visibly rather than panic |
| User travels without changing Strata configuration | Fixed-offset interpretation remains unchanged |
| User changes offset later | New sessions use new policy; historical grouping remains persisted |

## Certification

TEMPORAL-001 adds unit and process coverage for:

- future timestamps;
- backward and forward live clock jumps;
- ordinary wall jitter;
- suspend-like agreement between wall and monotonic elapsed time;
- explicit long-interval acceptance;
- fixed-offset behavior across DST seasons;
- travel/configuration projection changes;
- preservation of active state on rejected legacy stops;
- all existing SQLite authority, lifecycle, recovery, and configuration gates.
''')

Path("notebook/NOW.md").write_text(r'''---
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
''')

Path("notebook/work/ISSUE-RECONCILIATION-001.md").write_text(r'''---
id: ISSUE-RECONCILIATION-001
kind: work
state: active
authority: working
created: 2026-08-01
updated: 2026-08-01
---

# ISSUE-RECONCILIATION-001 — post-SQLite queue

The original issue descriptions predate the completed SQLite migration. Their acceptance criteria remain useful, but code-path premises must be reverified.

## Current disposition

| Issues | Disposition | Next owner |
|---|---|---|
| #8, #9, #11 | Completed and closed by the SQLite program. | none |
| #21 | Completed by AUTHORITY-001: CLI/TUI share one fail-closed startup configuration gate with explicit `--ignore-config`. | none |
| #25 | Completed by TEMPORAL-001: monotonic live duration, checked UTC recovery, fixed-offset civil authority, clock-jump refusal, and persisted historical day grouping. | none |
| #15 | Complete profile identity, isolation, and deliberate runtime switching remain open. | AUTHORITY-002 or a later profile unit |
| #4, #23, #27 | Interval-boundary allocation, misleading sunrise semantics, and zero-duration policy are the next coupled temporal risks. | TEMPORAL-002 |
| #2, #12 | SQLite preserves project strings, but the complete project/classification product contract must be reconciled before closure. | DOMAIN-001 |
| #1, #14, #17, #28, #3 | Reporting and export semantics/documentation. | REPORT-001 |
| #5, #10, #13 | SQLite integrity, active authority, and category archival likely satisfy substantial portions; verify every criterion before closing or rewriting. | reconciliation audit |
| #6, #7, #16, #18, #26 | Sediment conservation/topology/rendering remain conceptually coupled. | SEDIMENT-001 |
| #19, #20, #24 | Interaction modes, terminal cleanup, and keymap truth remain independent of SQLite. | INTERACTION-001 |
| #22 | Active draft versus category metadata remains a domain/UI distinction. | DOMAIN-002 |

## Immediate action

Implement TEMPORAL-002 for issues #4, #23, and #27. Clock authority is now explicit; the remaining question is how truthful intervals are divided at boundaries, named, and represented when their duration is zero.
''')

Path("notebook/work/RELIABILITY-001-persistence-and-audit-remediation.md").write_text(r'''---
id: RELIABILITY-001
kind: work
state: active
authority: working
created: 2026-08-01
updated: 2026-08-01
---

# RELIABILITY-001 — persistence and audit remediation

## Completed persistence program

SQLITE-001 through SQLITE-012 completed schema, strict legacy import, explicit activation, shared CLI/TUI authority, transactional runtime coordination, deterministic interchange, maintenance, visible recovery, exhaustive fault certification, and legacy-evidence custody. Issue #8 closed at 9/9 acceptance criteria.

## Completed authority units

### AUTHORITY-001 — issue #21

- one validated startup configuration is shared by CLI and TUI;
- invalid authority/time settings fail before writable authority opens;
- `--ignore-config` is the explicit deliberate-default bypass;
- TUI reload retains the last valid settings on failure.

### TEMPORAL-001 — issue #25

- live duration is monotonic and committed with the same elapsed value;
- persisted absolute timestamps are UTC;
- live wall/monotonic skew above five seconds blocks mutation and preserves active state;
- future timestamps and unrepresentable monotonic reconstruction fail visibly;
- cross-process intervals above seven days require explicit CLI confirmation;
- configured fixed offset owns new civil projection and operational-day allocation;
- persisted operational-day keys own historical report grouping after later setting changes;
- the policy is explicitly fixed-offset, not IANA/DST.

Profile switching remains separate under issue #15.

## Remaining order

1. **TEMPORAL-002** — issues #4, #23, #27.
2. **DOMAIN-001** — issues #2 and #12 residuals.
3. **REPORT-001** — issues #1, #14, #17, #28.
4. **SEDIMENT-001** — issues #6, #7, #16, #18, #26.
5. **INTERACTION-001** — issues #19, #20, #24.

## Closure discipline

Each issue must be reconciled against current main. Close only when every acceptance criterion is supported by merged behavior or when the issue is explicitly rewritten to isolate the residual defect.
''')
