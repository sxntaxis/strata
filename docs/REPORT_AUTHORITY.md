# Report and export authority

Status: accepted authority; BALANCE-LAYER-DETAIL-UX-001 candidate natively validated, owner review pending
Last reviewed: 2026-09-27

## Purpose

Reports and exports are projections over Strata's canonical chronological ledger. They expose active uncertainty and operational-day allocation without rewriting canonical sessions.

## Current product vocabulary

The interactive historical/report surface is named **Balance**. Day, week, and month are presets over report windows; arbitrary inclusive operational-day windows are already valid reporting semantics and HISTORY-001 exposes them directly in the TUI rather than creating a second report engine.

## Range semantics

`--today`, `--week`, and `--month` use the configured operational-day/calendar policy. `report --from YYYY-MM-DD --to YYYY-MM-DD` selects an inclusive range of operational-day keys; reversed or incomplete ranges fail.

Balance consumes the same inclusive `ReportWindow` authority, but its visible interval chrome is a boundary projection: the left date is the first included operational day and the right date is the exclusive boundary one day after the last included operational day. Thus internal `2026-09-21..2026-09-21` displays `Sep 21 – Sep 22`, and internal `2026-09-21..2026-09-27` displays `Sep 21 – Sep 28`. This is presentation/interaction semantics only; report filtering remains inclusive over `ReportWindow.start..=ReportWindow.end`.

The Balance `[` and `]` actions select the visible start or exclusive-end boundary. Left/Right then moves that selected boundary by one operational day while preserving a minimum one-day window and preventing the included end from moving later than the current operational day. Without a selected boundary, Left/Right retains whole-window navigation.

The inline `r` From/To editor uses the same visible boundary convention. It accepts `YYYY-MM-DD`, requires `From < To`, converts `To` to the inclusive internal end by subtracting one day, and then applies one ordinary `ReportWindow`; it does not synthesize a second TUI-specific interval model. A custom window may still be shifted backward or toward the present by its own inclusive span, but forward whole-window navigation never extends past the current operational day.

A canonical session remains one row. Exact overlap slices contribute only the seconds that belong inside the selected operational-day range.

## Balance layer-detail ledger projection

Entering one Balance layer projects the selected report window as an editable ledger without creating a second report
engine. Completed sessions that cross operational-day boundaries remain one row: all slices of the same canonical
session that intersect the selected window are aggregated for row display, with the first included boundary and last
included boundary retained for temporal presentation. The provisional active generation may also project as a row,
but because it has no completed persisted session identity it is not an existing-entry edit target.

The row presentation is deliberately responsive to interval scope. A single-operational-day window uses three evenly
distributed data regions: Tag, Time, and signed Effect. A multi-day window uses four evenly distributed regions: Tag,
Date, Time, and signed Effect. Date remains adjacent to Time rather than becoming a leading identity column. If an
entry crosses civil dates, the temporal middle region may expand to an explicit `start → end` pair so both date/time
boundaries remain truthful.

Above the ledger, the layer hero shows the selected layer's signed `balance_seconds` sum and a nested influence meter.
The parent Balance meter supplies the displacement envelope. Let `P` be the absolute parent marker offset in cells;
for a non-zero layer contribution `L`, let `S` be the aggregate magnitude of all contributions on `L`'s polarity side.
The layer marker magnitude is the rounded, clamped `P * |L| / S`, and its direction is the sign of `L`. Therefore a
layer marker can oppose the parent direction but can never exceed the parent's distance from equilibrium. When the
parent marker is at equilibrium, layer influence also renders at equilibrium. This meter is a drill-down attribution
of the parent displacement, not an independently normalized hypothetical Balance.

## Explicit historical correction

Balance browsing remains a projection until the user explicitly enters a ledger mutation. Historical mutation has
two deliberate entry points: the Balance-wide **Log past activity…** assignment operation and the scoped layer-detail
ledger editor. The global operation states one past interval plus one existing target layer; the layer ledger can also
edit one completed canonical entry in place or add a new entry already scoped to the selected layer.

HISTORY-001C established the first safe transactional primitive by reclassifying a positive sub-interval of one
completed Idle session while conserving canonical whole seconds, regenerating affected `daily-contribution`
artifacts atomically, validating current active mass, and reloading memory only after SQLite commit.

HISTORY-001D generalizes that primitive to arbitrary historical assignment with the following product contract:

- `From < To <= now`; historical correction can never create future time.
- The requested interval may cross zero, one, or many completed canonical rows and may also intersect the current
  active generation.
- Idle is transparent for collision policy. Existing time already classified to the requested layer is also
  non-conflicting.
- Intersecting a different explicit layer produces a collision preview and requires confirmation. Confirmation is
  valid only for the exact observed canonical plan; changed authority must be previewed again.
- Applying the assignment carves the requested interval out of every intersecting completed row, inserts only the
  missing requested-layer chronology, and preserves valid before/after fragments without double-counting. True
  chronological gaps are writable; a pre-existing Idle row is not required.
- The current selected layer and description are protected. If corrected past time intersects the active
  generation, SQLite may rebase/restart that generation while preserving what the user is doing now. Changing the
  current activity remains an explicit live switch/stop action.
- If the requested layer already is the selected live layer and the assignment makes history continuously that
  layer up to the active boundary, the active start may move backward.
- SQLite completed chronology, active-generation/checkpoint authority, affected `daily-contribution` artifacts,
  and the in-memory projection publish coherently.

A layer-ledger **existing-entry edit** is narrower than generalized assignment: it preserves the selected completed
session's layer and stable row identity while replacing its Tag and complete start/end boundary. It requires
`start < end <= now`, rejects any overlap with another completed canonical session or the protected current activity,
replaces affected daily-contribution projections atomically, and reloads memory only after SQLite commit. It does not
silently carve neighboring history. A layer-ledger **Add entry** instead reuses generalized assignment, including its
collision preview/replacement semantics, with the selected layer fixed and the ledger Tag persisted.

The transaction rejects pre-existing overlapping canonical history rather than rewriting ambiguous double-counted
authority. Whole-second allocation continues to use retained boundary provenance and the existing cumulative
allocator, including fractional UTC boundaries and operational-day cuts.

HISTORY-001D established ledger truth without changing sediment. HISTORY-001E extends that same assignment transaction with bounded current-pile reconciliation: canonical seconds reclassified from one existing category to another request an in-place transfer of retained source-category sediment into the target category. True-gap seconds create no current grains, and prior clears may limit how much source mass remains available. Missing visual mass never blocks the ledger correction and unrelated categories are never consumed to force the current pile to equal historical accounting. First-write authentic day-end snapshots remain immutable.

The Balance-wide inline editor assigns layer and From/To time. It still does not infer or expose a historical Tag, so
rows inserted through that global route use an empty description. The layer-detail ledger has a separate explicit Tag
field for its add/edit row and persists exactly that description. Active-generation rebasing preserves the persisted
live description.

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
