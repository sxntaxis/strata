---
id: BALANCE-UX-003
kind: work
state: candidate
authority: working
created: 2026-09-27
updated: 2026-09-27
---

# BALANCE-UX-003 — shared overlay comfort geometry and summary polish

## Owner-approved corrections

Runtime review of BALANCE-UX-002 kept the content-aware width floor but rejected content-only height as the general modal rule. The Layer/Strata modal's proportional empty space remains desirable because it tracks terminal size and aspect ratio rather than hugging the rows.

The accepted overlay rule is:

```text
desired outer width  = max(content minimum, terminal width / 3)
desired outer height = max(content minimum, terminal height / 3)
then clamp to the configured frame margin.
```

Content is therefore a hard minimum and viewport proportion is the comfort floor. Layer and the default Balance summary are the first consumers of one shared Ratatui `Rect` authority. Settings and command palette retain their existing special ratios in this slice.

## Balance presentation

The default Balance frame again follows the selected row's layer color, matching the established Layer-modal behavior. Idle therefore naturally produces its configured idle/white frame; the frame is not a permanent theme-border blue or a permanent white.

Historical snapshot provenance remains model authority but leaves normal Balance chrome. No `day-end checkpoint`, `latest saved checkpoint`, `derived preview`, reconstruction, or idle-policy sentence occupies the summary body.

The instrument keeps its approved negative/net/positive hierarchy. The side aggregates now color only their sign glyphs: negative `-` uses `theme_error`, positive `+` uses `theme_success`, and the duration digits remain `theme_status`. The centered net remains fully polarity-colored. The equilibrium `┼` remains exactly as subdued as the inactive axis.

## Deferred boundary

Custom-range gesture design remains explicitly deferred. This unit does not decide Shift/Ctrl/Alt arrow semantics, change text entry, or address Caps Lock normalization.
