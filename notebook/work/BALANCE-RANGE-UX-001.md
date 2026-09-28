---
id: BALANCE-RANGE-UX-001
kind: work
state: probe
authority: working
created: 2026-09-27
updated: 2026-09-27
summary: "Measure raw modifier-key delivery through the owner's real terminal + tmux path before choosing a custom-range gesture."
---

# BALANCE-RANGE-UX-001 — modifier input probe

## Owner direction

Custom-range interaction remains deliberately unresolved. The owner prefers a seamless modifier-driven gesture over a text-entry/tab gate if the terminal path delivers modifiers reliably, and has observed that Caps Lock can make ordinary letters appear equivalent to Shift-modified input.

Do not redesign production range controls from assumptions. First capture the raw `crossterm::event::KeyEvent` values produced by the real terminal + tmux environment.

## Probe

`examples/key_event_probe.rs` is a standalone diagnostic only. It does not route through Strata actions, modify the keymap, or alter production behavior. Run it from the same shell/tmux context used for Strata:

```bash
cargo run --example key_event_probe
```

Capture at least:

1. `Shift+Left`
2. `Shift+Right`
3. `Ctrl+Left`
4. `Ctrl+Right`
5. `Alt+Left`
6. `Alt+Right`
7. lowercase `a`
8. Caps Lock on + `a`
9. Caps Lock off + `Shift+a`

Record each emitted `code`, `modifiers`, `kind`, and `state`. Include any extra escape/prefix events if the terminal/tmux path emits them.

## Decision gate

No custom-range gesture is authorized by this unit alone. After the real-path capture, choose the simplest gesture whose modifier identity is stable enough for production and explicitly account for Caps Lock normalization.

Current `r` range editing remains unchanged until that decision.

## Non-goals

This probe does not change:

- Balance keybindings;
- current `r` editor behavior;
- report/date semantics;
- modal layout;
- sediment behavior;
- Settings/keymap schema.
