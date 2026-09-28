---
id: BALANCE-TAG-FILTER-UX-001
kind: work
state: candidate
authority: working
created: 2026-09-28
updated: 2026-09-28
summary: "Add independent semicolon multi-tag attribution, canonical dim autocomplete, and contextual OR filtering to Balance Layer Detail without hiding chronology or changing the D071 recorded-time denominator."
---

# BALANCE-TAG-FILTER-UX-001 — multi-tag attribution and Layer Detail filtering

## Owner decision

A layer remains the stable activity/category axis. Tags are finer attribution facets inside that layer.

- One session Tag field may contain multiple tags separated with `;`, for example `Renzo; Anibal`.
- Canonical display/storage uses `; `, trims empty values, deduplicates case-insensitively, and reuses the known canonical spelling when available.
- Tags do **not** divide the session. A one-hour `Renzo; Anibal` entry remains one one-hour wall-clock ledger row while attributing the full hour independently to Renzo and to Anibal. This supports genuinely parallel billable work.
- Existing `sessions.description` / active description remains the storage authority; this unit adds no schema, split-time ledger, or second persistence authority.
- Per-layer tag history remains suggestion/canonical-spelling metadata. It stores individual tags rather than semicolon combinations.

STRATA-D072 records this accepted semantic contract.

## Completion interaction

Known tags are offered as a **dim suffix**, so text already typed by the user and text merely suggested by Strata are visually distinct.

- Completion operates only on the current semicolon token.
- Existing tags already present in the description are not suggested again.
- In the inline ledger editor, `Tab` accepts a visible Tag completion before advancing to the next field.
- Layer-modal Left/Right tag cycling remains the existing lightweight completion route and operates on the current semicolon token rather than replacing the full multi-tag description.
- Commit canonicalizes spelling and remembers the individual component tags.

## Layer Detail filter

`balance_filter`, default `f`, is contextual to Layer Detail. Filtering never deletes rows from view.

- With no selected facets, every ledger row renders normally.
- `f` on the selected row toggles one focused tag facet into or out of an explicit filter set.
- The filter set uses **OR** semantics. `Renzo + Anibal` means a row matches when it carries Renzo **or** Anibal.
- Nonmatching rows remain in chronological position but render dim. They remain selectable, so `f` can directly bring their tags into the filter.
- A multi-tag row has temporary tag focus while filtering. Left/Right moves the focus among that row's individual tags; `f` toggles only the focused tag. Strata does not generate or cycle through combinatorial tag subsets.
- An empty stored tag remains filterable as `untagged`; ordinary row display still falls back to the layer name without writing that fallback to storage.
- Esc first leaves tag focus, then clears a non-empty filter, then resumes ordinary Layer Detail back/close behavior.
- The selected filter survives day/week/month/custom-window navigation while the same layer remains open.

## Filtered meter semantics

Filtering changes the Layer Detail contribution, not the shared scale.

- Filtered subtotal/meter numerator = signed `balance_seconds` of the **union of matching ledger rows**.
- Each canonical ledger row contributes once even when it contains multiple selected tags.
- Denominator remains the selected period's complete `summary.total_seconds`, including Idle, exactly as required by STRATA-D071.

Therefore a one-hour `Renzo; Anibal` row may legitimately report one attributable hour under a Renzo-only view and one attributable hour under an Anibal-only view. A combined `Renzo OR Anibal` Layer Detail filter still projects one hour, not two, because the meter is projecting selected chronological rows rather than summing per-tag billing attribution.

## Authority reconciliation included

The validated PR #109 base already has STRATA-D071 accepted and native-green. `docs/INTERACTION_AUTHORITY.md` still described the superseded D069 parent-envelope/polarized meter and older separate historical editor. This unit reconciles that accepted document with D071 and the already owner-approved unified ledger-correction interaction; it does not change those runtime semantics.

## Implementation candidate

- Added one tag grammar helper for parse/canonicalize/completion/current-token replacement and membership checks.
- Existing Layer and ledger Tag entry paths share the canonical semicolon grammar.
- Per-category completion candidates include persisted tag history plus tags discoverable from existing/current sessions; older history values containing semicolons are normalized into individual candidates on read/merge.
- Added configurable `balance_filter` / `f` action and Layer Detail filter/focus state.
- Nonmatching rows are dimmed, not removed.
- Filtered Layer Detail hero uses the filtered row-union numerator and unchanged D071 denominator; the hero labels active filter facets.
- CLI tag predicates now match individual tags inside a multi-tag description so `--tag Renzo` continues to mean Renzo after multi-tag storage is introduced. A semicolon query means all listed tags for the CLI predicate; this is distinct from the interactive Layer Detail OR-set.
- No SQLite schema or migration change.

## Native validation facts — 2026-09-28

- `cargo fmt --all -- --check`, strict Clippy, `cargo test --all-features`, and `cargo run -- --help` pass. The suite reports 573 library tests and 24 integration/process tests passed; 20 ignored; 0 failed.
- Focused tests cover case-insensitive parse/dedup/canonicalization, current-token completion/acceptance, tag membership and CLI-style all-tag predicates, OR row-union filtering, untagged filtering, and configurable `balance_filter` binding.
- A freshly imported eight-layer test profile at `/mnt/Tokyo/Lab/.tmp/opencode/strata-balance-tag-filter-ux-001-test-profile` (UUID `8ed894a9-2810-459c-aceb-0ef5ce2961f4`) was used for 120×40 TUI smoke. Known Office tags offered the dim completion; a committed `Renzo; Anibal y Renzo` Add appeared as one 15-minute row. `f` focused Renzo; Left/Right switched the multi-tag row's focused facet and `f` added the second OR facet. The filtered subtotal updated once per matching row, and nonmatching chronology remained visible.
- The test profile SQLite doctor passed after app exit: schema v1, integrity and foreign-key checks healthy. The loaded source test profile was exported read-only and not modified by the smoke.

The owner accepted STRATA-D072, and it is now implemented and natively certified in the accepted decision index and report/interaction authorities. The exact implementation base was PR #109 head `7f7e56d4e7c6fc34b3438c099b49707c06b2edc4`; no schema change was made.

## Publication — 2026-09-28

- Published as draft PR [#110](https://github.com/sxntaxis/strata/pull/110), stacked on PR #109 (`work/balance-meter-layer-parity-001`).
- STRATA-D072 remains accepted; its implementation status is natively certified. SEDIMENT-016 remains separate.

## Owner-review follow-up

The native-green PR #110 behavior above is the historical validation baseline for D072. Subsequent owner review accepted STRATA-D073 in `BALANCE-BEHAVIOR-COHERENCE-001`: Tab no longer accepts completion, empty Tag segments no longer show a ghost suggestion, and ledger/Layer tag cycling now uses prefix-aware Left/Right semantics. D072's multi-tag attribution and OR-filter semantics themselves are unchanged.
