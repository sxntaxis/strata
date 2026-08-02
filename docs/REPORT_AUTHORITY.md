# Report and export authority

Status: accepted and certified
Implemented by: REPORT-001
Issues: #1, #14, #17, #28
Last reviewed: 2026-08-02

## Range semantics

Custom report ranges are inclusive operational-day ranges. `--from YYYY-MM-DD --to YYYY-MM-DD` requires both endpoints and rejects a start later than the end. Canonical sessions contribute only the overlap slices assigned to days inside the range.

## Active-session projection

Reports and ordinary JSON/ICS exports include the active interval by default as a provisional projection measured once at command execution. This projection does not mutate the ledger or create a completed session. `--completed-only` selects committed history only.

Provisional JSON rows are marked with `provisional: true`. Provisional ICS events carry `X-STRATA-PROVISIONAL:TRUE`. Zero-whole-second and idle intervals remain excluded from ordinary work output.

## Deterministic ordering

Report entries use a complete order:

1. elapsed duration descending;
2. normalized category name ascending;
3. category ID ascending.

Category exports sort by normalized name and ID. Session exports sort by authoritative UTC start, UTC end, and stable UID. Identical inputs therefore produce stable row order.

## ICS authority

ICS events use authoritative UTC start/end timestamps and stable UIDs. Text values escape backslash, newline, semicolon, and comma; content lines fold at 75 octets and output uses CRLF. Missing project identity is omitted rather than replaced with invented text. Sessions without authoritative UTC chronology fail ICS export rather than producing misleading floating times.

Generated output was accepted by two independent parsers: Python `icalendar` and `ics.py`.

## Boundaries

REPORT-001 does not change canonical ledger rows, persistence schema, project identity, TUI date editing, or sediment formation. Reports and exports remain projections over chronological authority.
