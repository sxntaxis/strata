---
id: INTERACTION-001C
kind: work
state: active
authority: working
created: 2026-08-02
updated: 2026-08-02
---

# INTERACTION-001C — keymap truth and command-atlas parity

## Issue

Issue #24: an action can appear unbound or disabled in configuration while remaining reachable through hard-coded F1 handling, hidden Confirm/Cancel/ReportToday fallbacks, or contextual modal remapping. The command atlas currently synthesizes those paths as if they were ordinary bindings.

## Selected contract

### Action state

Every action has exactly one configured state:

- `Bound` — one or more configurable direct keys;
- `Unbound` — no direct keys and no explicit prohibition;
- `Disabled` — explicitly prohibited as a configurable or contextual action.

A null physical-key entry removes that key only. `unbind_actions` means Disabled. An action with no remaining direct keys and no disabled marker is Unbound.

### Mandatory policy

`Ctrl-C → Quit` is the only mandatory process-level key. It is not an ordinary configurable binding, cannot be rebound or disabled, resolves before configured keys, and is labeled separately in the command atlas.

F1 is not mandatory. It remains a normal configurable default for `toggle_keybindings_help`; removing or disabling that action removes F1 behavior.

### Contextual policy

Current aliases become named policy entries rather than handler fallbacks:

- `main.confirm → open_layer_popup` when the target is Unbound;
- `main.cancel → switch_to_drift` when the target is Unbound;
- `main.karma_today → detach` when the target is Unbound;
- `report.detach → karma_today` always when the target is not Disabled.

Aliases are inherited/configured through `contextual_aliases`. A disabled target is never reached. An absent alias leaves the source action unchanged.

### Runtime routing

One keymap resolver returns direct, mandatory, contextual, or no action for an explicit input context. Event handlers execute the returned action without inspecting whether another action has keys. Physical F1 bypasses and hidden fallback conditionals are removed.

Modal-local text/edit controls remain owned by their explicit modal modes rather than masquerading as action bindings.

### Atlas truth

The command atlas displays:

- direct keys for Bound actions;
- `(unbound)` for Unbound actions;
- `(disabled)` for Disabled actions;
- mandatory Ctrl-C separately on Quit;
- named contextual aliases and their activation rule;
- editing controls that distinguish Disable from Unbind.

## Acceptance proofs

- disabled actions are unreachable through direct keys, aliases, and the palette;
- unbound actions remain distinct from disabled actions;
- null key removal can produce Unbound without Disabled;
- F1 obeys configured state;
- mandatory Ctrl-C always resolves to Quit and conflicts are rejected;
- each inherited alias reproduces current intended behavior only under its declared rule;
- removing or disabling an alias/target removes the contextual invocation;
- atlas labels exactly match runtime reachability;
- JSON editing and atlas editing preserve action state;
- all previous interaction, PTY, persistence, sediment, report, CLI, and TUI gates remain green.
