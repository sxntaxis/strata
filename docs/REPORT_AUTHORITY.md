# Report and export authority

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
- profile-bound SQLite authority paths;
- deterministic report and export tie-breakers;
- JSON schema version 2 and project propagation;
- authoritative UTC ICS, escaping, folding, stable UIDs, idle omission, and provisional marking;
- independent ICS parser acceptance;
- truthful week/month CLI help;
- all existing persistence, temporal, domain, recovery, CLI, and TUI gates.
