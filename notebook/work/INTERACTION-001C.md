---
id: INTERACTION-001C
kind: work
state: completed
authority: accepted
created: 2026-08-02
updated: 2026-08-02
---

# INTERACTION-001C — keymap truth and command-atlas parity

## Issue

Issue #24: an action could appear unbound or disabled in configuration while remaining reachable through hard-coded F1 handling, hidden Confirm/Cancel/ReportToday fallbacks, or contextual modal remapping. The command atlas synthesized those paths as if they were ordinary bindings.

## Accepted contract

### Action state

Every action has exactly one configured state:

- `Bound` — one or more configurable direct keys;
- `Unbound` — no direct keys and no explicit prohibition;
- `Disabled` — explicitly prohibited as a configurable or contextual action.

A null physical-key entry removes that key only. `unbind_actions` means Disabled. An action with no remaining direct keys and no disabled marker is Unbound. A manual configuration that binds and disables the same action is contradictory and rejected rather than resolved silently.

### Mandatory policy

`Ctrl-C → Quit` is the only mandatory process-level key. It is not an ordinary configurable binding, cannot be rebound or disabled, resolves before configured keys, and is labeled separately in the command atlas.

F1 is not mandatory. It remains a normal configurable default for `toggle_keybindings_help`; removing or disabling that action removes F1 behavior.

Mandatory Quit remains under persistence-recovery custody. During a persistence failure, Ctrl-C uses the same emergency-export-and-exit path as deliberate recovery Quit rather than bypassing recovery state or looping without exit.

### Contextual policy

Current aliases are named policy entries rather than handler fallbacks:

- `main.confirm → open_layer_popup` when the target is Unbound;
- `main.cancel → switch_to_drift` when the target is Unbound;
- `main.karma_today → detach` when the target is Unbound;
- `report.detach → karma_today` always when the target is not Disabled.

Aliases are inherited/configured through `contextual_aliases`. A disabled target is never reached. An absent alias leaves the source action unchanged.

### Runtime routing

One keymap resolver returns direct, mandatory, contextual, or no action for an explicit input context. Event handlers execute the returned action without inspecting whether another action has keys. Physical F1 bypasses and hidden fallback conditionals are removed.

Modal-local text/edit controls remain owned by their explicit modal modes rather than masquerading as action bindings.

### Atlas and palette truth

The command atlas displays:

- direct keys for Bound actions;
- `(unbound)` for Unbound actions;
- `(disabled)` for Disabled actions;
- mandatory Ctrl-C separately on Quit;
- named contextual aliases and their activation rule;
- editing controls that distinguish Backspace Disable from Delete Unbind;
- close, movement, and jump hints derived from the same configured keys used by runtime.

Disabled actions are removed from the command palette. Unbound actions remain executable through deliberate palette selection and are labeled as unbound rather than given invented keys.

## Certified proofs

- disabled actions are unreachable through direct keys, aliases, and the palette;
- unbound actions remain distinct from disabled actions;
- null key removal can produce Unbound without Disabled;
- contradictory Bound and Disabled configuration fails closed;
- F1 obeys configured state;
- mandatory Ctrl-C always resolves to Quit and configuration conflicts are rejected before persistence;
- mandatory Ctrl-C during persistence recovery exports recovery evidence and requests exit;
- each inherited alias reproduces intended behavior only under its declared rule;
- removing or disabling an alias/target removes the contextual invocation;
- atlas labels and control hints derive from runtime reachability;
- JSON editing and atlas editing preserve action state;
- all previous interaction, PTY, persistence, sediment, report, CLI, and TUI gates remain green.

Certification passed:

- formatting;
- strict Clippy with all targets/features and warnings denied;
- 181 library tests;
- 9 CLI lifecycle tests;
- 6 configuration-authority tests;
- 1 report-help regression test;
- 12 SQLite/TUI process tests;
- 2 temporal-authority tests;
- 3 terminal-lifecycle PTY process tests.

## Durable authority

- `docs/INTERACTION_AUTHORITY.md` records key state, mandatory and contextual policy, recovery-safe Quit, and atlas/runtime parity;
- `docs/ARCHITECTURE.md` assigns input resolution and visible interaction truth to the shared keymap/application boundary;
- STRATA-D038 through STRATA-D040 constrain key state, routing, and interface parity;
- `notebook/work/INTERACTION-001.md` closes the interaction-authority program;
- `notebook/work/ISSUE-RECONCILIATION-001.md` marks issue #24 complete.

## Boundary

This unit does not make modal-local text editing controls configurable and does not redefine future profile isolation, category metadata, or queued cross-authority mutation receipts.