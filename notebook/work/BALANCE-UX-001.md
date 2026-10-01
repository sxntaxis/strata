---
id: BALANCE-UX-001
kind: work
state: completed
authority: working
created: 2026-09-27
updated: 2026-09-27
---

# BALANCE-UX-001 — summary instrument

## Owner-approved direction

The default Balance summary is an instrument rather than a shortcut guide or generic report popup.

The accepted candidate hierarchy is:

```text
╭────────────────────── Balance ──────────────────────╮
│                                                    │
│ -01:01:05          -00:46:44          +00:14:21    │
│     └──────────●━━━━━━┼──────────────────┘          │
│                                                    │
│ ● positive layer                        +00:14:21  │
│ · idle                                    00:03:38  │
│ ○ negative layer                        -01:01:05  │
│                                                    │
╰────────────────── < Sep 27 > ─────────────────────╯
```

The mockup specifies hierarchy, not fixed terminal dimensions.

## Summary semantics

The instrument displays three presentation-only aggregates derived from the existing `BalanceReportSummary`:

- left: sum of negative `balance_seconds` entries;
- center: authoritative report `total_balance_seconds` (net);
- right: sum of positive `balance_seconds` entries.

Idle/neutral time remains a normal row and contributes zero to the signed aggregates. Existing Balance row ordering remains unchanged: positive/neutral layers, Idle, then negative layers under the current domain ordering.

The signed meter is normalized over polarized time only:

```text
negative_abs = abs(negative_total)
polarized = negative_abs + positive_total
position = (positive_total - negative_abs) / polarized
```

No polarized time places the dot at exact center. All-negative and all-positive states place it at the respective usable extremes. The meter is presentation only and creates no stored state.

## Ratatui geometry

The summary uses ordinary Ratatui primitives (`Block`, `Line`, `Span`, `Paragraph`, `List`, `Layout`, `Constraint`, `Rect`, `Alignment`, `Style`, `Modifier`). `Gauge`/`LineGauge` are intentionally not used because Balance is bipolar around a fixed equilibrium.

Normal instrument width prefers 45 terminal cells. Responsive widths preserve an odd width with three equal columns where possible (`45`, `39`, `33`, `27`). The left total is left-aligned, net centered, and positive total right-aligned in independent Ratatui rects. The meter uses the exact same width, so the middle value's center and the fixed equilibrium cell cannot drift apart through hand-counted spaces.

Very narrow terminals degrade to a centered net plus bounded meter rather than overlapping or panicking.

## Color authority

The candidate uses existing theme semantics only:

- summary frame: `theme_border`;
- `Balance`: bold `theme_foreground`;
- side totals: `theme_status`;
- net: `theme_error` / `theme_success` / `theme_status` by sign;
- inactive axis/endcaps: dim `theme_status`;
- active displacement and moving dot: error or success by polarity;
- equilibrium marker: `theme_foreground`;
- date/range: `theme_foreground`, with navigation chevrons in `theme_status` and unavailable newer navigation dimmed.

Category identity remains category-colored in the list. Metric polarity remains semantic error/success/status. Selected rows continue to use the shared contrast resolver introduced by UX-CONTRAST-001.

## Interaction hierarchy

The default summary removes visible Day/Week/Month/Range labels, side-border navigation arrows, and normal action hints such as `[L] Log past` or `Enter details`.

The currently selected report interval becomes the centered bottom-border object:

```text
< Sep 27 >
< Sep 21-27 >
< September 2026 >
```

Existing keyboard authority is unchanged: `d/t`, `w`, `m`, `r`, `l`, left/right, Enter, and Esc retain their current behavior. Help/Settings/command palette remain the discoverability surfaces for shortcuts.

## Explicit boundary

This unit does not redesign category detail/log view, range editing, historical activity editing, collision confirmation, report arithmetic, temporal boundaries, persistence, theme schema, category semantics, or sand physics. Those surfaces retain current safety guidance until separately reviewed.

## Result

The instrument is implemented on the contrast-authority branch lineage. It retains the domain's authoritative net, derives polarized sides from existing category rows, and renders a symmetric bipolar meter with responsive Ratatui layout. The only visible interaction change is the default summary hierarchy and interval control placement; detail/edit/history behavior remains as before.

Native verification passes `cargo fmt --all -- --check`, strict Clippy, `cargo test --all-features` (519 unit and 24 integration tests; 20 ignored), `cargo run -- --help`, and `git diff --check`. A PTY smoke on disposable profile `/mnt/Tokyo/Lab/.tmp/opencode/strata-contrast-001-smoke` opened Balance with Idle selected and observed black foreground on the white selection; navigating Balance exercised black and white contrast choices; Layer, Settings, and Command Palette selected views rendered and exited normally. No render-unit scaffold was needed because the contrast resolver tests plus this direct screen-output proof covered the selected-idle regression.
