# Report and export authority

Status: accepted and certified
Last reviewed: 2026-08-27

## Purpose

Reports and exports are projections over Strata's canonical chronological ledger. They expose active uncertainty and operational-day allocation without rewriting canonical sessions.

## Current product vocabulary

The interactive historical/report surface is named **Balance**. Day, week, and month are presets over report windows; arbitrary inclusive operational-day windows are already valid reporting semantics and HISTORY-001 exposes them directly in the TUI rather than creating a second report engine.

## Range semantics

`--today`, `--week`, and `--month` use the configured operational-day/calendar policy. `report --from YYYY-MM-DD --to YYYY-MM-DD` selects an inclusive range of operational-day keys; reversed or incomplete ranges fail.

A canonical session remains one row. Exact overlap slices contribute only the seconds that belong inside the selected operational-day range.

## Provisional active time

Unless `--completed-only` is supplied, reports and JSON/ICS export include the current active interval projected from its persisted UTC start to one snapshot time.

The provisional row:

- does not stop or mutate the active session;
- carries category identity and active description;
- uses the configured operational-day policy;
- is announced in human output;
- carries `provisional: true` in JSON;
- carries `X-STRATA-PROVISIONAL:TRUE` in ICS.

Future-dated active starts fail closed. Zero-whole-second intervals contribute no ordinary work row.

## Deterministic output

Report entries sort by elapsed time, case-insensitive category name, then category ID. Exported categories sort by name/ID. Exported sessions sort by authoritative UTC chronology and stable UID.

## JSON contract

General JSON export schema version 4 contains:

- optional repository numeric ID;
- stable UID;
- provisional flag;
- category ID and name;
- category `balance_effect`;
- description;
- civil display fields;
- elapsed seconds;
- authoritative UTC endpoints when known.

The retired independent `project` field is not part of schema 4. Schema 4 also retires the old exported `karma_effect` field name in favor of `balance_effect`. General JSON is a projection format; deterministic full-state interchange uses the separate SQLite portable-bundle contract.

## ICS contract

ICS work events use authoritative UTC `DTSTART`/`DTEND`, stable UIDs, one snapshot `DTSTAMP`, CRLF line endings, escaping, and line folding. Category/layer is the event summary. Description is emitted separately when present.

Idle/category ID `0` and zero-duration intervals are omitted. A completed session without authoritative UTC chronology cannot be safely represented and causes export to fail closed.

## Boundaries

Reporting does not mutate the ledger, fragment canonical sessions, or create a second grouping axis. Future grouping/filter concepts above layers require separate product evidence.
