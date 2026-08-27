# Report and export authority

Status: accepted and certified
Last reviewed: 2026-08-27

## Purpose

Reports and exports are projections over Strata's canonical chronological ledger. They expose active uncertainty and operational-day allocation without rewriting canonical sessions.

## Current product vocabulary

The interactive historical/report surface is named **Balance**. Day, week, and month are presets over report windows; arbitrary inclusive operational-day windows are already valid reporting semantics and HISTORY-001 exposes them directly in the TUI rather than creating a second report engine.

## Range semantics

`--today`, `--week`, and `--month` use the configured operational-day/calendar policy. `report --from YYYY-MM-DD --to YYYY-MM-DD` selects an inclusive range of operational-day keys; reversed or incomplete ranges fail.

Balance exposes the same arbitrary-window contract through its `range` mode. The inline From/To editor accepts `YYYY-MM-DD`, validates `from <= to`, and applies one `ReportWindow`; it does not synthesize a different TUI-specific interval model. A custom window may be shifted backward or toward the present by its own inclusive span, but forward navigation never extends past the current operational day.

A canonical session remains one row. Exact overlap slices contribute only the seconds that belong inside the selected operational-day range.

## Explicit historical correction

Balance browsing and reporting remain read-only projections. Historical mutation occurs only through explicit
committed correction commands.

HISTORY-001C adds the first safe mutation: **Log missed activity**. It operates only on one persisted completed
Idle session selected from Balance. The requested interval must be positive, must remain inside that canonical Idle
session, and must target an existing active non-Idle layer. The operation never inserts overlapping elapsed time.
Instead, one Idle row may become up to three chronological rows: Idle-before, corrected activity, and Idle-after.
The original row identity is retained by the fragment that keeps the original session start; newly created
fragments receive deterministic split identities and all replacement rows are marked with historical-correction
source provenance.

The SQLite publication is one `IMMEDIATE` transaction. Session splitting and replacement of every affected
`daily-contribution` artifact commit together. When an affected day also contains the current active generation,
the TUI supplies a stable-identity-qualified provisional preview; SQLite validates that preview against
`active_session` and includes its canonical slices in the replacement. A stale preview fails closed rather than
dropping live mass. The transaction preserves the original corrected session's canonical whole-second total,
including across operational-day boundaries. Current canonical sediment is not recolored by HISTORY-001C, and
first-write authentic day-end snapshots are not rewritten. Those visual semantics remain separate HISTORY-001E
and explicit-product-decision territory.

Non-Idle source reclassification is outside HISTORY-001C and fails closed; deliberate correction of already
classified activity belongs to HISTORY-001D.

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

Passive reporting does not mutate the ledger or create a second grouping axis. Explicit historical correction is a separate committed command governed by the constraints above; it may deliberately split a canonical session while conserving its elapsed-time truth. Future grouping/filter concepts above layers require separate product evidence.
