from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if text.count(old) != 1:
        raise SystemExit(f"expected exactly one match in {path}: {old[:80]!r}")
    target.write_text(text.replace(old, new, 1))


report_section = '''## Reporting and exports

Reports are projections over canonical ledger truth. Preset reports use the current operational day, configured week-to-date, or calendar month-to-date. Custom ranges are inclusive operational-day ranges:

```bash
strata report --from 2026-07-01 --to 2026-07-15
```

A running interval is included by default as provisional time and is identified explicitly in report output and exports. Use committed history only when required:

```bash
strata report --today --completed-only
strata export --format json --completed-only
```

JSON export schema version 2 includes stable event UIDs, authoritative UTC endpoints, and a `provisional` flag. ICS export uses those UTC endpoints and stable UIDs, emits CRLF-delimited RFC 5545 text with escaping and line folding, marks provisional events with `X-STRATA-PROVISIONAL:TRUE`, and excludes idle events. A legacy session without authoritative absolute chronology fails closed for ICS rather than inventing timestamps.

Week reports follow the configured first day of week. The current week is week-to-date; prior week offsets in the TUI are complete calendar weeks. Month reports use calendar months: the current month is month-to-date and prior offsets are complete prior calendar months.

The detailed contract is recorded in [`docs/REPORT_AUTHORITY.md`](docs/REPORT_AUTHORITY.md).

'''
replace_once("README.md", "## Persistence authority\n", report_section + "## Persistence authority\n")

architecture_report = '''\n\nREPORT-001 establishes projection authority:\n\n- custom report ranges are inclusive operational-day ranges and consume exact canonical overlap slices;\n- the active interval is included by default as a provisional projection without mutating or finalizing it;\n- `--completed-only` selects committed history for reports and JSON/ICS exports;\n- report and export ordering has complete deterministic tie-breakers;\n- JSON schema version 2 exposes stable UIDs, provisional state, and authoritative UTC endpoints;\n- ICS uses stable UIDs and UTC endpoints, applies RFC 5545 escaping, CRLF delimiters, and line folding, marks provisional events explicitly, and fails closed when absolute chronology is unavailable;\n- idle remains excluded from ordinary reports and ICS work events.\n\nThe detailed contract is `docs/REPORT_AUTHORITY.md`.\n'''
replace_once(
    "docs/ARCHITECTURE.md",
    "\n## Truth boundaries\n",
    architecture_report + "\n## Truth boundaries\n",
)
replace_once(
    "docs/ARCHITECTURE.md",
    "Persistence structure, startup configuration fallback, clock authority, interval boundaries, and session classification are no longer the primary risks. The next program is projection correctness:\n\n1. correct reporting ranges, provisional active time, ICS validity, and deterministic ordering;\n2. establish a conserved sediment model independent of viewport and mutable previews;\n3. complete interaction-mode and terminal-lifecycle contracts.\n",
    "Persistence, startup configuration, clock authority, interval boundaries, session classification, and report/export projection correctness are no longer the primary risks. The next programs are:\n\n1. establish a conserved sediment model independent of viewport and mutable previews;\n2. complete interaction-mode and terminal-lifecycle contracts.\n",
)

replace_once(
    "docs/DECISIONS.md",
    "| STRATA-D019 | CLI starts require explicit category classification; idle is explicitly selectable and omission never silently becomes idle. | implemented and certified |\n",
    "| STRATA-D019 | CLI starts require explicit category classification; idle is explicitly selectable and omission never silently becomes idle. | implemented and certified |\n"
    "| STRATA-D020 | Report ranges are inclusive operational-day projections over exact canonical overlap slices; reporting never fragments or mutates the owning session. | implemented and certified |\n"
    "| STRATA-D021 | The active interval is included by default in reports and exports as explicitly provisional state; `--completed-only` selects committed history. | implemented and certified |\n"
    "| STRATA-D022 | Report/export ordering is deterministic; ICS uses stable identities and authoritative UTC chronology with RFC 5545-safe serialization, and fails closed rather than inventing timestamps. | implemented and certified |\n",
)

Path("docs/REPORT_AUTHORITY.md").write_text('''# Report and export authority

Status: accepted and certified
Implemented by: REPORT-001
Issues: #1, #3, #14, #17, #28
Last reviewed: 2026-08-02

## Purpose

Reports and exports are projections over Strata's canonical chronological ledger. They must make active uncertainty, operational-day allocation, ordering, and adapter limitations visible without rewriting the underlying sessions.

## Range semantics

`--today`, `--week`, and `--month` use Strata's operational-day and calendar policy:

- today is the current operational day;
- week is the configured calendar week to date;
- month is the current calendar month to date.

`report --from YYYY-MM-DD --to YYYY-MM-DD` selects an inclusive range of operational-day keys. Both endpoints are required, and a reversed range is rejected. Canonical sessions remain singular; exact overlap slices contribute only their seconds inside the selected range.

## Provisional active time

A report or general JSON/ICS export takes one read snapshot. Unless `--completed-only` is supplied, an existing active interval is projected from its persisted UTC start to that snapshot time.

Provisional projection:

- does not stop, finalize, split, or otherwise mutate the active session;
- uses the active session's project and category identity;
- uses the configured operational-day policy for its derived slices;
- is announced in human-readable reports;
- carries `provisional: true` in JSON;
- carries `X-STRATA-PROVISIONAL:TRUE` in ICS.

A future-dated active start is rejected. An interval with no complete elapsed second contributes no provisional work row.

## Deterministic output

Report entries sort by descending elapsed seconds, then case-insensitive category name, then category ID. Exported categories sort by case-insensitive name and ID. Exported sessions sort by authoritative start UTC, end UTC, and stable UID. Repeated projections of the same committed state therefore do not depend on map or repository iteration order.

## JSON contract

General JSON export uses schema version 2. Session records include:

- optional numeric legacy/repository ID;
- stable UID;
- explicit provisional state;
- project and category identity;
- civil display fields;
- elapsed seconds;
- authoritative UTC start and end when known.

This general export is a projection format. Deterministic full-state interchange remains the separate SQLite bundle contract.

## ICS contract

ICS work events use authoritative UTC `DTSTART` and `DTEND`, stable UIDs, one export snapshot `DTSTAMP`, CRLF line endings, escaped text fields, and folded content lines. Independent `icalendar` and `ics.py` parsers certify generated output.

Idle/category ID 0 and zero-duration intervals are omitted from ICS work events. A legacy completed session without authoritative UTC chronology cannot be safely represented and causes ICS export to fail closed instead of combining ambiguous civil fields into fabricated timestamps.

## Help and terminology

CLI help describes the actual calendar policy: the configured current week and the current calendar month, not rolling seven- or thirty-day windows. README and regression tests carry the same contract.

## Boundaries

REPORT-001 does not add:

- a TUI custom-date editor;
- project-grouped or project-filtered reports;
- ledger mutation during projection;
- canonical-session fragmentation;
- sediment formation or conservation changes;
- a new full-state interchange format.

## Certification

REPORT-001 covers:

- inclusive custom ranges and reversed/incomplete-range refusal;
- provisional active inclusion and committed-only exclusion;
- legacy and SQLite authority paths;
- deterministic report and export tie-breakers;
- JSON schema version 2 and project propagation;
- authoritative UTC ICS, escaping, folding, stable UIDs, idle omission, and provisional marking;
- independent ICS parser acceptance;
- truthful week/month CLI help;
- all existing persistence, temporal, domain, recovery, CLI, and TUI gates.
''')

Path("notebook/NOW.md").write_text('''---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-02
authority: working
summary: Persistence, temporal, domain, and report/export authority are complete; sediment conservation now leads the frontier.
next: Implement SEDIMENT-001 for issues #6, #7, #16, #18, and #26 without weakening chronological ledger truth.
---

# NOW — Strata

## Current phase

The SQLite migration program, AUTHORITY-001, TEMPORAL-001, TEMPORAL-002, DOMAIN-001, and REPORT-001 are complete. Strata now has durable persistence, explicit clock and boundary semantics, preserved project/category identity, and truthful deterministic report/export projections.

The project is moving from **projection correctness** to **sediment conservation**.

## Accepted product baseline

- Strata is a continuous temporal ledger and an active timer.
- Time always passes; the baseline name is **idle**.
- Idle continues depositing sediment but is omitted from ordinary active-time accounting.
- Project and category are independent session axes.
- A CLI work session requires explicit category classification.
- Strata is general-purpose rather than freelancing-specific.
- Exact chronological history and accountable sedimentary history are both meaningful.
- Sediment is product function and artwork, not disposable decoration.
- One grain currently represents one elapsed second.
- Braille-cell color mixing is intentional.

## Verified technical baseline

- SQLite schema version 5 is authoritative after explicit activation.
- CLI and TUI share repository, runtime-coordination, configuration, temporal, and session-domain boundaries.
- Canonical sessions remain single identities while reports allocate exact overlap slices across operational days.
- Project and category survive legacy/SQLite lifecycle, TUI synchronization, custody export, JSON, and ICS.
- Reports accept inclusive custom operational-day ranges.
- Reports and general exports include active time by default as explicit provisional state; `--completed-only` selects committed history.
- Report and export ordering has deterministic tie-breakers.
- JSON schema version 2 carries stable UIDs, provisional state, and UTC endpoints.
- ICS uses authoritative UTC chronology, stable UIDs, RFC 5545-safe text serialization, and independent parser certification.
- Idle is excluded from ordinary active-time totals and ICS work events while remaining part of sediment history.
- Persistence, temporal, and projection failures remain fail-closed and recoverable.

## Completed post-migration units

- **AUTHORITY-001** — issue #21: shared validated settings and fail-closed CLI/TUI startup configuration.
- **TEMPORAL-001** — issue #25: clock roles, discontinuity handling, fixed-offset civil authority, and reproducible history.
- **TEMPORAL-002** — issues #4, #23, #27: overlap allocation, removal of false sunrise semantics, and zero-transition policy.
- **DOMAIN-001** — issues #2, #12: persisted project identity, explicit category requirement, and completed idle vocabulary migration.
- **REPORT-001** — issues #1, #3, #14, #17, #28: truthful ranges/help, provisional active projection, valid ICS, and deterministic ordering.

Complete profile isolation remains open under issue #15. Project CRUD, TUI project selection, project-grouped reporting, and a TUI custom-range editor remain future work.

## Active sequence

1. **SEDIMENT-001** — issues #6, #7, #16, #18, #26: conserved logical sediment independent of viewport and mutable previews.
2. **INTERACTION-001** — issues #19, #20, #24: explicit edit modes, terminal lifecycle guard, truthful keybinding policy.
3. Reconcile remaining partially satisfied issues #5, #10, #13 and later domain/profile work.

## Current risks

- Sediment rendering, resize, catch-up, and snapshots still lack one conservation model.
- The relationship between logical sediment mass, viewport capacity, and historical topology is not yet explicit.
- Interaction edit modes and terminal cleanup remain incompletely enforced.
- Complete profile switching/isolation remains open.

## Next

Implement **SEDIMENT-001**. Reconcile issues #6, #7, #16, #18, and #26 around one conserved logical sediment model. The visual projection may adapt to terminal geometry, but it must not silently create, discard, or reclassify accountable elapsed mass.
''')

replace_once(
    "notebook/work/ISSUE-RECONCILIATION-001.md",
    "| #1, #14, #17, #28, #3 | Reporting and export semantics/documentation. | REPORT-001 |",
    "| #1, #3, #14, #17, #28 | Completed by REPORT-001: inclusive operational-day ranges, truthful calendar help, provisional active projection, valid ICS, and deterministic ordering. | none |",
)
replace_once(
    "notebook/work/ISSUE-RECONCILIATION-001.md",
    "Implement REPORT-001 for issues #1, #14, #17, and #28. Canonical ledger, interval, project, and category truth are now stable; reporting and export projections must become complete without rewriting them.",
    "Implement SEDIMENT-001 for issues #6, #7, #16, #18, and #26. Reporting is now a truthful projection over stable ledger truth; sediment must gain an equally explicit conservation model without becoming a competing time authority.",
)
replace_once(
    "notebook/README.md",
    "The SQLite authority migration is complete. The active frontier is configuration/profile authority, followed by temporal correctness, reporting semantics, and sediment conservation.",
    "Persistence, temporal, domain, and report/export authority are complete. The active frontier is sediment conservation, followed by interaction-mode and terminal-lifecycle correctness.",
)

Path("tests/report_help.rs").write_text('''#![cfg(target_os = "linux")]

use std::process::Command;

#[test]
fn report_help_describes_calendar_periods_not_rolling_windows() {
    let output = Command::new(env!("CARGO_BIN_EXE_strata"))
        .args(["report", "--help"])
        .output()
        .expect("Strata help process should run");
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).expect("help should be UTF-8");
    assert!(help.contains("Show the current operational week"), "{help}");
    assert!(help.contains("Show the current calendar month"), "{help}");
    assert!(!help.contains("last 7 days"), "{help}");
    assert!(!help.contains("last 30 days"), "{help}");
}
''')

for temporary in [
    ".github/workflows/report001-source.yml",
    ".github/workflows/report001-tests.yml",
    ".github/workflows/report001-ics-parse.yml",
    ".github/workflows/report001-close.yml",
    "tools/report001-tests.patch.b64",
    "tools/report001.patch.b64",
    "tools/report001-close.py",
]:
    Path(temporary).unlink(missing_ok=True)
