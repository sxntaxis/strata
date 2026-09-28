---
id: BALANCE-LEDGER-CORRECTION-UX-001
kind: work
state: candidate
authority: working
created: 2026-09-28
updated: 2026-09-28
summary: "Unify Balance ledger Add/Edit on one historical-correction transaction, treat unowned time as implicit Idle, and confirm only collateral ledger mutation with a centered Before/After card."
---

# BALANCE-LEDGER-CORRECTION-UX-001 — unified ledger correction

## Owner decision

The inline ledger work in BALANCE-LAYER-DETAIL-UX-001 is the new interaction for creating and correcting recorded activity. It is not a second feature beside the older `Log past activity…` command-strip editor.

- `+ Add entry…` is the canonical UI for creating a ledger entry, including past activity.
- `balance_log_activity` / `l` is a shortcut into that same Add-entry flow for the selected Balance layer.
- The old Layer/From/To footer editor is retired rather than maintained as a parallel historical-entry interface.
- Existing-entry Edit and new-entry Add must use one historical-correction planner/executor rather than independent overlap/correction implementations.

The operation receives an optional source session for Edit. Planning conceptually removes that source, establishes the requested target interval, computes collateral effects, and publishes the complete correction atomically.

## Timeline ownership and Idle

Unowned canonical time is treated as **implicit Idle** even when SQLite does not materialize a large explicit Idle session row.

Consequences:

- shrinking/removing/moving an explicit entry releases its vacated time to Idle;
- assigning previously unowned/Idle time to a layer changes ownership from Idle to that layer;
- overwriting another explicit layer transfers ownership from that layer to the target layer;
- an edit that changes only its selected source entry and implicit Idle does not require a warning.

This owner decision supersedes the conservative BALANCE-LAYER-DETAIL-UX-001 edit rule that rejected any overlap with another completed session. Collateral explicit records may be carved after explicit confirmation.

## Sand reconciliation

Historical correction derives a category ownership delta from the complete before/after correction and applies that delta to retained current sediment using the existing fungible recolor doctrine.

Examples:

- shrink Work: `Work → Idle`;
- expand Work into Idle: `Idle → Work`;
- overwrite Leisure with Work: `Leisure → Work`.

Recolor remains bounded by retained physical mass, preserves coordinates/topology/FIFO/physics state, does not replay physics, and does not mutate authentic historical day-end photographs. The correction does not claim per-grain temporal provenance.

## Collateral confirmation

The warning surface exists only when the requested correction changes **other explicit recorded ledger state** (or relevant current recorded activity). Changes confined to the selected source entry and Idle apply directly.

The warning replaces the layer hero temporarily; the edited draft remains visible in the ledger. It is a centered Before/After comparison, not a second modal and not a textual collision dump.

Single collateral entry:

```text
╭────────────── Another entry will change ──────────────╮
│                                                       │
│           BEFORE                    AFTER              │
│                                                       │
│  Leisure  11:33–12:00              11:45–12:00        │
│                                                       │
│                Enter apply · Esc back                 │
╰───────────────────────────────────────────────────────╯
```

A split uses multiple AFTER rows. Complete removal displays `removed`. Multiple collateral entries share the same BEFORE/AFTER columns and use the plural title `N other entries will change`.

Do not expose collision-plan terminology, session IDs, SQLite details, grain counts, or `+/-` diff notation. Dates appear only when needed to disambiguate the selected report/cross-day interval.

## Implementation direction

The candidate removes the separate `edit_historical_session` and `log_historical_activity` public TUI paths in favor of one `apply_historical_correction` operation with:

- optional source session identity;
- target category, description, and full start/end timestamps;
- active-generation preview/custody;
- exact confirmation token;
- collateral Before/After projections;
- one atomic session rewrite, daily-contribution rebuild, checkpoint update, and retained-sand reconciliation.

The source session retains stable identity where the corrected target interval requires a replacement row. Neighboring explicit rows are carved into valid prefix/suffix fragments as needed.

## Preservation

This pass does not reopen Balance arithmetic, boundary semantics, the nested layer influence meter, selected-layer colors, one-cell modal spacing, snapshot authority, SQLite schema, or SEDIMENT-016 physics/corridor authority.

## Validation state

Assistant-side environment has no Rust toolchain. Current implementation is staged for native validation; only repository/static hygiene can be checked here. Native formatter, strict Clippy, all-feature tests, CLI smoke, focused correction/sediment tests, and disposable-profile TUI persistence remain required before publication or promotion.
