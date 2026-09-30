# Report and export authority

Status: accepted authority; STRATA-D071/D072/D073/D074 are natively certified; STRATA-D075-D081 are the prior reviewed frontier; STRATA-D082 is the active owner-approved implementation candidate pending native validation
Last reviewed: 2026-09-30

## Purpose

Reports and exports are projections over Strata's canonical chronological ledger. Reporting-cycle allocation may slice contribution accounting, but it never replaces the canonical civil identity of a session.

## Current product vocabulary

The interactive historical/report surface is named **Balance**. Day, week, and month are presets over report windows; arbitrary inclusive reporting-cycle windows are already valid reporting semantics and HISTORY-001 exposes them directly in the TUI rather than creating a second report engine. The configured fixed boundary is user-facing **Reporting cutoff**; the buckets it defines are **Reporting cycles**. Internal persisted operational-day vocabulary remains schema/implementation terminology.

## Range semantics

`--today`, `--week`, and `--month` use the configured reporting-cutoff/calendar policy. `report --from YYYY-MM-DD --to YYYY-MM-DD` remains the CLI canonical form and selects an inclusive range of reporting-cycle keys; reversed or incomplete ranges fail.

Balance consumes the same inclusive `ReportWindow` authority, but its visible interval chrome is a boundary projection: the left date is the first included reporting cycle and the right date is the exclusive boundary one cycle after the last included reporting cycle. Thus internal `2026-09-21..2026-09-21` displays `Sep 21 – Sep 22`, and internal `2026-09-21..2026-09-27` displays `Sep 21 – Sep 28`. This is presentation/interaction semantics only; report filtering remains inclusive over `ReportWindow.start..=ReportWindow.end`.

The Balance `[` and `]` actions select the visible start or exclusive-end boundary and begin one preview transaction over the whole opening range. Left/Right previews a one-reporting-cycle move; Shift+Left/Right previews one civil month with normal end-of-month clamping. `[` and `]` may switch endpoints inside that same transaction. Enter accepts the resulting range; Esc restores the complete opening range/navigation state. Without a selected boundary, Left/Right retains whole-window navigation and Shift+Left/Right has no period-cycling meaning.

The inline From/To range editor remains an unbound configurable/palette action using the same visible boundary convention. Its TUI Date fields use the shared permissive civil-date grammar (canonical/compact numeric input, English month names, deterministic missing-component defaults, and overflow carry), normalize on semantic action, require `From < To`, convert `To` to the inclusive internal end by subtracting one day, and then apply one ordinary `ReportWindow`; it does not synthesize a second TUI-specific interval model. Normal keyboard custom-range construction uses `[` / `]`. A custom window may still be shifted backward or toward the present by its own inclusive span, but forward whole-window navigation never extends past the current reporting cycle.

A canonical session remains one row. Exact overlap slices contribute only the seconds that belong inside the selected reporting-cycle range.

## Balance layer-detail ledger projection

Entering one Balance layer projects the selected report window as an editable ledger without creating a second report engine. A visible row represents one **canonical entry identity**, never a disposable reporting slice. If a completed session intersects several reporting cycles it may therefore appear in each relevant view with the same full canonical civil Date/Time, while its signed Effect in each view is only the exact overlap contribution to that selected report window. Edit or delete from any occurrence operates on that same canonical entry everywhere. The provisional active generation is also a first-class stable ledger identity; it renders its civil start through open-ended `now`, permits Tag/start-boundary editing, and deleting it removes that active generation before immediately returning Strata to Idle.

Row geometry is alignment-first rather than equal-cell. A single-day ordinary row uses Tag · Time · Effect with Time exactly centered and mirrored outer regions. A row that shows Date uses Tag · Date · Time · Effect with Date/Time as a symmetric inner pair and Tag/Effect mirrored outside them. Cross-day civil entries retain separate Date and Time spans. The measured Tag region owns a small bounded golden-ratio-derived gutter before the next region; this is breathing room only and never redefines the symmetric column anchors. Width authority is the data actually visible in the current viewport plus the active draft and required instrument chrome; off-screen ledger rows do not widen or compress the current view. Scrolling to genuinely wider visible data may widen the overlay and leaving it may let the overlay shrink. A long Tag does not force all other cells to its width. Focus highlight, cursor, completion, and filter chrome do not resize the modal. During a single-day edit only the active row may expose Date while surrounding rows retain their ordinary three-cell form. Complete ledger values are preferred whenever the terminal can hold them; ellipsis is reserved for genuine hard-width pressure, and an active field is never ellipsized but instead uses a caret-following viewport when physically necessary.

Each ordinary ledger row begins with the layer marker in the first region. If the persisted Tag is empty, presentation uses the neutral `—` placeholder without writing it into storage. The synthetic `+ Add entry…` row uses `+` itself in that exact marker position. Text editing uses the real blinking block caret rather than literal bracket characters. Ordinary Layer navigation selection uses the Layer color with automatically contrasting text; an active editable value or focused multi-tag facet uses the theme-owned `ui.selection` treatment on top of that row, and generic non-Layer selection uses the same theme role. Entering edit mode does not arbitrarily recolor unrelated cells. The Time en dash is structural chrome and is never a caret position. The Effect cell never disappears merely because editing begins: it previews the draft overlap against the current report window and falls back to a dim `—` only when the draft is genuinely uninterpretable. Accepting a Tag ghost completion with Right places the caret at the end of the accepted current token.

A non-empty Tag field may carry multiple independent attribution tags. The existing session description remains the
only persisted field; canonical serialization trims/deduplicates tags case-insensitively and writes them as an ordered
`; `-separated list such as `Renzo; Anibal`. The tags do not partition the row's elapsed time. A one-hour row with
`Renzo; Anibal` represents one hour of wall-clock chronology and one full hour attributable to Renzo plus one full hour
attributable to Anibal when attribution is examined per tag. Per-category tag history is completion/canonical-spelling
metadata, not a second chronological ledger.

Above the ledger, the layer hero shows the selected layer's signed `balance_seconds` sum and the same recorded-time
meter used by the default Balance summary. The full horizontal line represents `summary.total_seconds` for the selected
period, including Idle. The global Balance marker projects `summary.total_balance_seconds / summary.total_seconds`, and
each Layer Detail marker independently projects that layer's signed `balance_seconds / summary.total_seconds` onto the
same radius. Opposite-sign layers may therefore sit on the opposite side of equilibrium from the global result, and a
globally balanced period does not collapse non-zero layer contributions to the center. Projection uses the original
time values and rounds once at the terminal-cell boundary. Exact zero alone occupies equilibrium; a representable
non-zero value receives at least one cell of displacement toward its sign. The meter uses the available instrument
width rather than imposing a fixed 45-cell cap. Layer Detail is therefore a directly comparable projection on the same
scale, not a subscale derived from the parent marker's already-quantized displacement.

Layer Detail may additionally project an explicit tag filter without hiding chronology. The filter is an OR-set of
individual tag facets. Rows matching any selected tag remain normal and nonmatching rows remain present but dimmed; an
untagged row is a distinct internal filter facet whose visible Tag/filter label is the neutral `—` placeholder. The
filtered hero subtotal and marker sum each matching ledger row exactly once, even when one row carries or matches more
than one selected tag. Only that numerator changes: the meter denominator remains the complete selected-period
`summary.total_seconds` required by STRATA-D071. Consequently a one-hour `Renzo; Anibal` row in a one-hour period may
be fully attributable to both tags separately, while a combined `Renzo OR Anibal` Layer Detail filter still projects
that one ledger row as one hour rather than two. The selected tag filter persists while the user navigates report
periods inside the same layer.

The active filter label is separate instrument chrome below the meter and serializes selected tags with the same `; ` grammar used by Tag input. De-emphasis is semantic presentation: nonmatches reuse theme-derived secondary/status color roles plus terminal dimming rather than assuming a particular gray RGB value. The ledger remains chronological, with a presentation-only blank row before `+ Add entry…` when vertical room permits. Layer Detail width is solved from ledger data under the symmetric alignment geometry above; cursor/focus/autocomplete chrome is excluded, while the actual draft value is data and may widen the modal. Filter focus uses the nested theme `ui.selection` highlight over the Layer-themed selected row rather than bracket characters, so activating `f` neither changes modal width nor steals cells from a Tag that previously fit. No editor-specific preferred-detail-width floor exists beyond the ordinary Balance/Strata comfort rules. Tag remains left-aligned, temporal values centered on their prescribed anchors, and Effect right-aligned. Ellipsis is therefore a last-resort physical-terminal fallback, never the representation of an active editable value.

## Balance summary visibility and Layer deletion

Balance summary omits exact-zero rows as presentation only: an ordinary Layer appears only when its displayed `balance_seconds` is non-zero for the selected period, while Idle appears only when its elapsed seconds are non-zero. Changing the report window may therefore reveal or hide Layers without mutating them. Selection follows stable Layer identity when possible and falls to a remaining visible row when the selected identity disappears from the projection.

`Ctrl+x` on the Balance summary requests permanent deletion of the selected non-Idle Layer through the universal `WARNING` overlay. `Go back` is selected by default; Enter executes the selected row and Esc takes the safe exit. Layer deletion, exact-row ledger-entry deletion, and collateral-overlap decisions share this overlay language instead of bespoke cards or accent-blue action text. A confirmed deletion removes that Layer's canonical sessions and Tag history and removes it from normal active/archived product surfaces; an active target is first switched safely to Idle. Every retained current and persisted historical sediment representation of that CategoryId is reclassified to Idle in the same authority operation. Coordinates, mass, topology, pending FIFO/counts, chronology, and simulation metadata are preserved; only Layer classification changes, and snapshot source identity is refreshed after the rewrite. SQLite retains a hidden non-restorable tombstone only to prevent CategoryId reuse. In Layer Detail, `Ctrl+x` deletes only the selected persisted ledger entry and does nothing on the synthetic `+ Add entry…` row.

Layer color is presentation attached to stable Layer identity rather than a frozen property of each sediment photo. Recoloring an existing Layer therefore changes how that CategoryId renders in both live and historical sediment without changing stored grain coordinates or mass. Permanent deletion is the explicit exception that changes classification itself: the deleted CategoryId becomes Idle everywhere it still appears in sediment authority.

## Explicit historical correction

Balance browsing remains a projection until the user explicitly enters a ledger mutation. The layer-detail ledger is
the canonical historical-correction surface. `+ Add entry…` creates past activity for that layer, Confirm on an existing
completed row edits that stable source entry. The synthetic Add row is the normal keyboard route; `balance_log_activity` remains available as an unbound configurable/palette action rather than a default `l` shortcut. The former separate Balance-wide Layer/From/To editor is retired.

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

Layer-ledger **Edit** and **Add entry** use one historical-correction planner/executor. Edit supplies the selected
completed source identity so that source may be replaced while preserving its stable identity where the corrected row
continues; Add has no source identity. Both require `start < end <= now`. Time owned only by the selected source and
implicit Idle may be reassigned without a warning. If the correction would carve, split, remove, or otherwise change a
different explicit completed entry or relevant current recorded activity, Strata presents the exact Before/After
collateral preview and requires confirmation for that observed plan before committing. Unowned chronology is implicit
Idle and need not exist as a materialized Idle row. Whole-second allocation continues to use retained boundary
provenance and the existing cumulative allocator, including fractional UTC boundaries and operational-day cuts.

HISTORY-001D established ledger truth without changing sediment. HISTORY-001E extends that same assignment transaction with bounded current-pile reconciliation: canonical seconds reclassified from one existing category to another request an in-place transfer of retained source-category sediment into the target category. True-gap seconds create no current grains, and prior clears may limit how much source mass remains available. Missing visual mass never blocks the ledger correction and unrelated categories are never consumed to force the current pile to equal historical accounting. First-write authentic day-end snapshots remain immutable under ordinary historical correction; STRATA-D076 permanent Layer deletion is the explicit Layer→Idle classification exception.

The unified layer-detail Add/Edit row owns the historical Tag field and persists its canonical description, including
semicolon-separated multi-tag attribution under STRATA-D072. `balance_log_activity` enters this same row when invoked through configuration/palette rather than creating a second global editor. Active-generation rebasing preserves the persisted live description.

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
