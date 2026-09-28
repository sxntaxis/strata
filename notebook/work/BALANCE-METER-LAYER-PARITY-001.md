---
id: BALANCE-METER-LAYER-PARITY-001
kind: work
state: candidate
authority: working
created: 2026-09-28
updated: 2026-09-28
summary: "Put Balance and Layer Detail on one recorded-time meter scale, use available meter width with nonzero displacement, restore marker/tag/editor ledger parity, and give Layer Detail the full Balance period selector."
---

# BALANCE-METER-LAYER-PARITY-001 — shared meter and layer-detail parity

## Owner decision

Balance and Layer Detail are two views of the same instrument. Their horizontal meter must therefore use one semantic scale for the selected period instead of making Layer Detail a subscale of the already-rounded parent displacement.

- The full meter line represents `summary.total_seconds`: all recorded time represented in the selected period, including Idle.
- The global Balance marker is `summary.total_balance_seconds / summary.total_seconds` projected onto the meter radius.
- A Layer Detail marker is that layer's signed `balance_seconds / summary.total_seconds` projected onto the same radius.
- Global and layer markers are independent projections on the same scale. A globally positive period may therefore contain a negative layer whose detail marker sits left of equilibrium, and a globally balanced period does not collapse nonzero layer markers to the center.
- Projection is performed from the original time values and rounded once at the terminal-cell boundary. The previous parent-displacement envelope/double-quantization model is retired.
- The meter consumes the available instrument width instead of stopping at 45 cells. Exact zero alone occupies equilibrium; any representable non-zero signed value receives at least one cell of displacement toward its sign after projection.

The Balance side totals remain the negative aggregate, net, and positive aggregate; Idle is neutral there but remains part of the meter's recorded-time denominator.

## Accepted-authority adjudication

The owner explicitly adjudicated the conflict on 2026-09-28. STRATA-D069 is superseded by STRATA-D071, and `docs/REPORT_AUTHORITY.md` now records the shared recorded-time denominator, independent global/layer projection, available-width meter, and one-cell minimum displacement for non-zero values. The former parent-envelope rule is retired authority rather than a pending gate.

## Marker parity

Balance uses the same layer marker grammar as the Strata layer modal:

- negative layer: `◯`;
- positive or neutral/Idle layer: `●`.

Layer Detail ledger rows carry the same marker before the tag, colored with the layer color when unselected. Selected rows retain contrast-safe text against the layer background. `+ Add entry…` uses `+` itself as the synthetic row marker, in the exact marker column occupied by `●`/`◯`; it is not indented past that slot.

An empty persisted ledger description remains empty in storage. Presentation falls back to the selected layer name, so an untagged Cinema row renders `Cinema` instead of a visually blank Tag cell.

## Inline ledger editor

Add and existing-entry Edit retain the correction transaction and full timestamp semantics already implemented, but their TUI projection is a single ledger row rather than the obsolete three-line `Tag:` / `From:` / `To:` form. The active field is bracketed in place, `+` remains the Add marker, an existing row retains its layer marker, and the editor uses the same Tag/Date/Time/Effect geometry as ordinary rows. Same-day windows omit the date cell; multi-day windows retain it; cross-civil-date edits use one explicit `start → end` temporal cell. The Effect cell stays empty while the draft chronology is being edited rather than displaying a stale pre-edit effect.

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

## Native validation facts — owner-review amendment, 2026-09-28

- `cargo fmt --all -- --check`, strict Clippy, `cargo test --all-features`, and `cargo run -- --help` pass. The suite reports 566 library tests and 24 integration/process tests passed; 20 ignored; 0 failed.
- Focused meter tests cover the shared recorded-time denominator, independent opposite-sign layer/global projections, noncollapsed layer positions when the global net is zero, a non-zero contribution in a three-cell meter, and expanded instrument widths.
- A freshly rooted imported test profile at `/mnt/Tokyo/Lab/.tmp/opencode/strata-balance-owner-review-inline-001-test-profile-r2` (UUID `3eaac84a-72c8-4f3e-8d71-f1b71ebabe73`) with eight layers was exercised at 120×40, 80×24, and 40×14. Balance/month showed the net marker against recorded time including Idle. Layer Detail displayed positive/neutral `●` and negative `◯` row markers, layer-name fallback for empty Tags, and outward-aligned edge cells. Week/month switching, `[` + Left boundary movement, Esc clearing, command-palette range-start selection, and `r` editing all retained Layer Detail.
- The `r` editor uses the bottom border in place of the period footer while active and has enough width to keep its fields/instructions visible. In the 40×14 editor, the active date is shown in full ISO form on the same inline row; time/date focus moves between fields without opening another panel.
- On the loaded profile, an empty Tag was edited to `Inline proof`, committed, and observed again in the ledger. It remained visible after process restart. SQLite doctor passed after apply and restart: schema v1, integrity and foreign keys healthy; the source profile used to create the portable fixture was not mutated.

STRATA-D071 supersedes D069 and is now accepted and natively certified. `docs/DECISIONS.md` and `docs/REPORT_AUTHORITY.md` are updated; SEDIMENT-016 remains separate.

The owner-reviewed amendment is natively validated and was fast-forwarded onto the existing draft PR #109. SEDIMENT-016 remains separate.

## Publication — 2026-09-28

- Published as draft PR [#109](https://github.com/sxntaxis/strata/pull/109), stacked on PR #108 (`work/balance-ledger-correction-ux-001`).
- Owner-review amendment is based on validated PR #109 head `2919966f3ea245b3c97ce1a38d549d037b875e2e` and fast-forwarded the draft PR to `be03e9023a97c8517a82e5f556156d83c7f0cd29`.
