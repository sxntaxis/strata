---
id: BALANCE-LAYER-DETAIL-UX-001
kind: work
state: candidate
authority: working
created: 2026-09-27
updated: 2026-09-27
summary: "Turn Balance layer detail into a coherent editable ledger with a minimal signed-sum hero and parent-relative influence meter."
---

# BALANCE-LAYER-DETAIL-UX-001 — layer ledger detail

## Owner decision

The drill-down reached by confirming a layer in Balance is the **Balance layer detail**. It remains inside the same Balance modal instead of becoming a separate form or second reporting surface.

The accepted presentation removes inherited aggregate chrome that does not belong to one layer:

- frame title is the layer name;
- no global Balance net in the upper-right;
- no duplicate interval at the upper-left;
- no side-border navigation arrows;
- no permanent `Log past / Enter edit / Esc back` instruction footer;
- the same explicit start/exclusive-end interval remains centered on the bottom border.

The body is the layer's ledger. Existing completed rows are edited in place; a synthetic final `+ Add entry…` row creates historical activity already scoped to that layer. `balance_log_activity` / `l` is contextualized to the same Add path while inside layer detail. The command palette follows the same current-view behavior.

## Hero

The layer detail reuses the parent Balance summary rhythm without cloning the three-number aggregate instrument.

The hero contains only:

1. the layer's signed Balance contribution for the selected report window, centered; and
2. one equilibrium meter below it.

The detail meter is **nested attribution**, not an independently normalized meter. The parent Balance marker's absolute distance from equilibrium is the maximum visual envelope for every layer detail in that same report window.

For a non-zero layer contribution `L`, let `P` be the absolute parent marker offset and `S` be the aggregate magnitude of the layer's polarity side. The detail magnitude is `round(P * |L| / S)`, clamped to `P`, and rendered on the side indicated by `L`'s sign. A layer can therefore pull opposite the global net, but its marker can never appear farther from equilibrium than the parent marker. If the parent is at equilibrium, layer influence renders at equilibrium too.

This meter answers **how much this layer participates in the parent Balance displacement**, not where Balance would land if the layer existed alone.

## Ledger rows

The row is treated as evenly distributed data rather than a left-heavy table.

For a one-day report window, ordinary rows use three equal regions:

`Tag | Time | Effect`

For a multi-day report window, ordinary rows use four equal regions:

`Tag | Date | Time | Effect`

Date stays immediately adjacent to Time. A cross-day entry is the exception: its two middle quarters become one explicit temporal span, e.g. `Sep 27 23:40 → Sep 28 00:20`, while Tag and Effect retain the outer regions.

Normal row time is minute precision. The inline editor exposes seconds.

Multiple operational-day slices from one canonical session project as one ledger row in the selected window. The active provisional row may be visible but has no completed stable session identity and is not an existing-entry edit target.

## Inline edit/add grammar

Confirm on an existing completed row edits that row in place. Confirm on `+ Add entry…` creates a temporary in-row draft rather than opening another modal.

Editable data is:

- Tag;
- start date/time;
- end date/time.

The active field is bracketed. Tab/BackTab changes field, Enter validates/commits, Esc cancels, and mandatory Ctrl-C remains the only application-level escape while the editor owns input.

Same-day entries share one visible date field; setting end time earlier than start time expresses the following civil day. Existing cross-day entries expose both date boundaries. The storage model remains complete start/end timestamps rather than a lossy `Date + From + To` interpretation.

Existing-entry edits preserve the layer and stable session identity and fail closed if the requested interval overlaps another completed canonical session or protected current activity. Add uses the established generalized historical-assignment collision transaction and persists the Tag entered in the layer ledger.

Archived layers remain browse/edit surfaces but do not expose `+ Add entry…`.

## Preservation

This candidate does not change:

- Balance report arithmetic;
- inclusive internal `ReportWindow` authority or exclusive visible end-boundary projection;
- parent Balance category ordering or meter normalization;
- selected-layer frame color;
- one-cell modal inset and Balance vertical rhythm;
- historical snapshot authority;
- SQLite schema;
- sediment runtime or SEDIMENT-016 authority.

## Validation state

The authored candidate includes focused domain/state/meter tests, including canonical cross-day row projection and the invariant that layer influence never exceeds the parent displacement envelope. Native formatter, strict Clippy, all-feature tests, TUI smoke, and SQLite doctor remain required on the owner's Rust environment before promotion.
