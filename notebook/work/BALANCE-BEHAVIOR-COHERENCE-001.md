---
id: BALANCE-BEHAVIOR-COHERENCE-001
kind: work
state: candidate
authority: working
created: 2026-09-28
updated: 2026-09-28
summary: "Make Balance ledger input, tag cycling, period-boundary acceleration, filtering chrome, and responsive Layer Detail geometry follow one coherent interaction grammar."
---

# BALANCE-BEHAVIOR-COHERENCE-001 — Balance input and presentation coherence

## Exact base

This pass starts from the natively certified draft PR #110 head `f8c397e71f3633c2e952be6c63c94902a6e6ecc4`. STRATA-D071 and STRATA-D072 remain unchanged semantic authority.

## Owner decision

STRATA-D073 records the owner-reviewed coherence pass:

- Tab/Shift+Tab are exclusively forward/backward field navigation in the inline ledger editor. Autocomplete never steals Tab.
- A Tag segment shows no completion while empty. After the first typed character, a dim completion may appear. Left/Right cycles known tags for the current semicolon segment, restricted to the typed prefix when one exists; an exact known tag cycles the full available known-tag set. Prior semicolon segments remain untouched.
- Date and Time remain separate Tab fields. Date Left/Right adjusts ±1 day and Shift+Left/Right ±7 days. Time Left/Right adjusts ±1 minute and Shift+Left/Right ±60 minutes. Arithmetic carries/borrows through hours/days and preserves seconds. Up/Down remains unclaimed.
- Time typing is forgiving: hour-only, compact HHMM/HMM, optional seconds, and colon forms are accepted; seconds are not required merely to enter a minute-precision value.
- With a Balance range boundary selected, Shift+Left/Right is the accelerated form of plain Left/Right: ±7 operational days instead of ±1, subject to the same range safety limits. Existing no-handle period cycling remains intact.
- Active filter state moves to a dedicated line below the meter and uses canonical `; ` separation. Nonmatching rows are more strongly de-emphasized through existing theme semantic roles plus terminal DIM, never a hardcoded gray.
- A presentation-only blank row separates chronology from `+ Add entry…` when height permits.
- Layer Detail may request more width for cross-day temporal spans or long tags while retaining the existing compact target for ordinary content and existing terminal-margin clamps.

## Implementation candidate

- Shared tag cycling now preserves the current typed-prefix constraint and is used by both the ordinary Layer modal and the Balance ledger editor.
- Empty current tag segments suppress ghost completion.
- Ledger key resolution makes Tab/BackTab navigation unconditional; Left/Right and Shift+Left/Right operate the active field without claiming Up/Down.
- Ledger time parsing accepts compact/partial forms and normalizes valid values when leaving the field. Temporal arrow adjustment operates on complete civil timestamps so overflow/underflow is real timestamp arithmetic.
- Balance range-boundary movement supports a bounded seven-step accelerated path without changing the existing one-day primitive.
- Filter chrome is rendered on a dedicated instrument row. Filter labels use `; `.
- Nonmatching rows resolve to the theme's existing subdued/status presentation and DIM modifier. No theme schema is extended.
- Layer Detail preferred width distinguishes compact ordinary rows from cross-day/long-tag rows, and Add receives a collapsible separator row.
- No schema, persistence, D071 meter denominator, D072 OR-filter semantics, historical-correction transaction, or output data shape changes.

## Native validation facts — 2026-09-28

- `cargo fmt --all -- --check`, strict Clippy, `cargo test --all-features`, and `cargo run -- --help` pass. The suite reports 581 library tests and 24 integration/process tests passed; 20 ignored; 0 failed.
- Focused tests cover compact/partial time parsing, canonical normalization, minute/hour/day arithmetic with second preservation, prefix-restricted tag cycling, empty-token completion suppression, accelerated boundary movement, and theme-role filter presentation.
- A freshly imported test profile at `/mnt/Tokyo/Lab/.tmp/opencode/strata-balance-behavior-coherence-001-test-profile-final` (UUID `adc71878-5643-4348-b638-f2e5561547e6`) was manually exercised at 120×40. Empty Tag showed no ghost completion; typing `R` produced a dim known-tag suffix; Left/Right cycled the matching tags; Tab moved to Date without accepting completion. Compact `650` input normalized to `06:50` when leaving the field; minute and Shift-hour adjustments preserved seconds. Shift+Left on a selected start boundary moved it back seven operational days while remaining in Layer Detail.
- SQLite doctor passed after PTY exit: schema v1, integrity and foreign keys healthy. The source profile used to create the portable test fixture was exported read-only and not mutated.

STRATA-D073 is accepted, implemented, and natively certified in the decision, report, and interaction authorities. This coherence pass changes no schema and leaves D071/D072 semantics and SEDIMENT-016 separate.

## Publication — 2026-09-28

- Published as draft PR [#111](https://github.com/sxntaxis/strata/pull/111), stacked on PR #110 (`work/balance-tag-filter-ux-001`).
- STRATA-D073 is accepted and natively certified; this PR changes no schema and keeps SEDIMENT-016 separate.
