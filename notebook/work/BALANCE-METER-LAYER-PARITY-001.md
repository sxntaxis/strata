---
id: BALANCE-METER-LAYER-PARITY-001
kind: work
state: candidate
authority: working
created: 2026-09-28
updated: 2026-09-28
summary: "Put Balance and Layer Detail on one recorded-time meter scale, restore layer-marker parity, align ledger edge cells outward, and give Layer Detail the full Balance period selector."
---

# BALANCE-METER-LAYER-PARITY-001 — shared meter and layer-detail parity

## Owner decision

Balance and Layer Detail are two views of the same instrument. Their horizontal meter must therefore use one semantic scale for the selected period instead of making Layer Detail a subscale of the already-rounded parent displacement.

- The full meter line represents `summary.total_seconds`: all recorded time represented in the selected period, including Idle.
- The global Balance marker is `summary.total_balance_seconds / summary.total_seconds` projected onto the meter radius.
- A Layer Detail marker is that layer's signed `balance_seconds / summary.total_seconds` projected onto the same radius.
- Global and layer markers are independent projections on the same scale. A globally positive period may therefore contain a negative layer whose detail marker sits left of equilibrium, and a globally balanced period does not collapse nonzero layer markers to the center.
- Projection is performed from the original time values and rounded once at the terminal-cell boundary. The previous parent-displacement envelope/double-quantization model is retired.

The Balance side totals remain the negative aggregate, net, and positive aggregate; Idle is neutral there but remains part of the meter's recorded-time denominator.

## Accepted-authority gate

This supplied owner candidate explicitly conflicts with accepted STRATA-D069 and `docs/REPORT_AUTHORITY.md` lines 40–46: accepted behavior divides the global meter by polarized time and derives Layer Detail from the rounded parent displacement envelope. The candidate instead divides global and layer values directly by all recorded period time, including Idle. Those accepted records remain unchanged in this working candidate. Before promotion, the owner must explicitly adjudicate whether this candidate supersedes D069 and the report-authority meter section.

## Marker parity

Balance uses the same layer marker grammar as the Strata layer modal:

- negative layer: `◯`;
- positive or neutral/Idle layer: `●`.

Layer Detail ledger rows carry the same marker before the tag, colored with the layer color when unselected. Selected rows retain contrast-safe text against the layer background. `+ Add entry…` has no fake layer marker; it is indented by the marker slot so its label aligns with ledger tags.

## Alignment rule

For row-like data regions, exterior cells align toward the containing walls and interior cells remain centered:

- leftmost cell: left aligned;
- rightmost cell: right aligned;
- interior cells: centered.

This applies to Layer Detail ledger rows and the collateral Before/After comparison. Balance's hero totals already follow the same left/center/right rule.

## Layer Detail period parity

The Balance period selector remains active inside Layer Detail without leaving the selected layer:

- `d/t`, `w`, `m`, and `r` keep their Balance semantics;
- `[` / `]` select the start/end boundary in Layer Detail too;
- Left/Right moves the selected boundary by one operational day;
- without a selected boundary, Left/Right shifts the whole interval;
- Esc clears a selected boundary before leaving Layer Detail;
- changing the period keeps the same layer and recomputes its sum, marker, and ledger;
- the bottom `< start – end >` boundary footer remains the shared visible period control in ordinary Layer Detail; while the `r` editor is active, its From/To controls temporarily occupy that footer to avoid overlapping titles.

Command-palette start/end boundary actions preserve Layer Detail rather than returning to the Balance summary.

## Preservation

This pass does not change historical-correction semantics, collateral-confirmation custody, SQLite schema, sediment correction/recolor doctrine, report-window inclusive internal semantics, or SEDIMENT-016 authority.

## Native validation facts — 2026-09-28

- `cargo fmt --all -- --check`, strict Clippy, `cargo test --all-features`, and `cargo run -- --help` pass. The suite reports 564 library tests and 24 integration/process tests passed; 20 ignored; 0 failed.
- Focused meter tests cover the shared recorded-time denominator, independent opposite-sign layer/global projections, and noncollapsed layer positions when the global net is zero.
- A freshly rooted imported test profile with eight layers was exercised at 120×40. Balance/month showed the net marker projected against recorded time including Idle; Layer Detail displayed positive/neutral `●` and negative `◯` row markers, aligned the left/right outer ledger cells, and retained the selected layer through Week, Month, `[` + Left boundary movement, Esc boundary clearing, and command-palette range-start selection.
- Opening `r` inside Layer Detail initially exposed overlapping bottom-border titles. The period footer now yields to the explicit From/To editor while that editor is active, and the modal grants it enough width to keep the full dates and instructions visible; PTY smoke confirmed the layer and ledger remain present.
- The isolated test profile's SQLite doctor passed after the smoke: schema v1, integrity and foreign-key checks healthy. The source profile used to create the portable test fixture was not mutated.

The implementation is natively validated as a candidate. Owner review must first resolve the accepted D069/report-authority meter conflict; SEDIMENT-016 remains separate.
